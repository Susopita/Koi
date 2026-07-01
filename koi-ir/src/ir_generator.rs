use crate::ast::ASTNode;
use crate::builtins::{BuiltinKind, builtin_kind};
use crate::ir::{BasicBlock, IRFunction, IRProgram, Instruction};
use crate::types::Type;
use std::collections::HashMap;

/// Lowers the (already-typed, already-lifted) AST to HIR instructions.
///
/// `if` and `loop` get real multi-block SSA codegen (`Branch`/`Jump`/`Phi`)
/// regardless of position -- an `if` used as an expression branches to a
/// `then`/`else` block and merges the result with a `Phi` in a join block;
/// a `loop` branches to a header block whose `Phi` carries the loop
/// variable across iterations via a back-edge. This works whether the
/// `if`/`loop` is in tail position or not, since `generate_expr` always
/// leaves its result usable in whatever block is now current -- callers
/// don't need to know if new blocks got created underneath them.
///
/// Struct/pointer/array operations and calls through a function-valued
/// parameter (e.g. `apply-func`'s `f`) get their own dedicated instructions
/// (`Alloc`/`GetField`/`GetIndex`/`AddrOf`/`Deref`/`CallIndirect`) rather
/// than being encoded as synthetic `Call`s to made-up names.
pub struct IRGenerator<'a> {
    functions: &'a HashMap<String, Type>,
    struct_fields: &'a HashMap<String, Vec<(String, Type)>>,
    temp_counter: usize,
    label_counter: usize,
    blocks: Vec<BasicBlock>,
    current_label: String,
    current_instructions: Vec<Instruction>,
    scopes: Vec<HashMap<String, (String, String)>>,
}

impl<'a> IRGenerator<'a> {
    pub fn new(
        functions: &'a HashMap<String, Type>,
        struct_fields: &'a HashMap<String, Vec<(String, Type)>>,
    ) -> Self {
        IRGenerator {
            functions,
            struct_fields,
            temp_counter: 0,
            label_counter: 0,
            blocks: vec![],
            current_label: "entry".to_string(),
            current_instructions: vec![],
            scopes: vec![],
        }
    }

    pub fn generate_program(&mut self, program: &ASTNode) -> Result<IRProgram, String> {
        let children = match program {
            ASTNode::Program { children } => children,
            other => return Err(format!("expected top-level program, got {other:?}")),
        };

        let mut functions = vec![];
        for child in children {
            if let ASTNode::FunctionDef {
                name,
                parameters,
                body,
                ..
            } = child
            {
                functions.push(self.generate_function(name, parameters, body)?);
            }
            // StructDef: nothing to emit at the instruction level for this MVP.
        }

        Ok(IRProgram {
            ir_type: "hir".to_string(),
            functions,
        })
    }

    fn generate_function(
        &mut self,
        name: &str,
        parameters: &[(String, Option<String>)],
        body: &ASTNode,
    ) -> Result<IRFunction, String> {
        self.temp_counter = 0;
        self.label_counter = 0;
        self.blocks.clear();
        self.current_label = "entry".to_string();
        self.current_instructions.clear();
        self.scopes.clear();
        self.scopes.push(HashMap::new());

        let (param_types, return_type) = match self.functions.get(name) {
            Some(Type::Function {
                params,
                return_type,
            }) => (params.clone(), (**return_type).clone()),
            _ => (vec![Type::Int64; parameters.len()], Type::Int64),
        };

        let mut ir_params = vec![];
        for ((pname, _), pty) in parameters.iter().zip(param_types.iter()) {
            let ty_str = pty.mangled_name();
            // Parameters are already-materialized values at function
            // entry: their own name doubles as their "temp".
            self.declare(pname, pname.clone(), ty_str.clone());
            ir_params.push((pname.clone(), ty_str));
        }

        let (result, _) = self.generate_expr(body)?;
        self.push_instruction(Instruction::Return {
            value: Some(result),
        });
        self.finish_block();

        Ok(IRFunction {
            name: name.to_string(),
            return_type: return_type.mangled_name(),
            parameters: ir_params,
            blocks: std::mem::take(&mut self.blocks),
        })
    }

    fn generate_expr(&mut self, node: &ASTNode) -> Result<(String, String), String> {
        match node {
            ASTNode::Literal {
                literal_type,
                value,
                ..
            } => {
                let ty = literal_ir_type(literal_type);
                let result = self.new_temp();
                self.push_instruction(Instruction::Const {
                    result: result.clone(),
                    value: value.clone(),
                    ty: ty.clone(),
                });
                Ok((result, ty))
            }

            ASTNode::Variable { name, line, column } => {
                if let Some(binding) = self.lookup(name) {
                    return Ok(binding);
                }
                if let Some(ty) = self.functions.get(name) {
                    // A bare reference to a top-level/lifted function name
                    // (what a zero-capture lifted lambda becomes). There's
                    // no first-class function value in this instruction
                    // set, so the name itself stands in as a placeholder
                    // (a real backend would need a function-pointer here).
                    let return_ty = match ty {
                        Type::Function { return_type, .. } => return_type.mangled_name(),
                        other => other.mangled_name(),
                    };
                    return Ok((name.clone(), return_ty));
                }
                Err(format!(
                    "Undefined variable '{name}' at line {line}, column {column}"
                ))
            }

            ASTNode::Call {
                function,
                arguments,
                ..
            } => self.generate_call(function, arguments),

            ASTNode::LetBinding { bindings, body, .. } => {
                self.scopes.push(HashMap::new());
                for (name, value) in bindings {
                    let (temp, ty) = self.generate_expr(value)?;
                    self.declare(name, temp, ty);
                }
                let result = self.generate_expr(body);
                self.scopes.pop();
                result
            }

            ASTNode::IfExpr {
                condition,
                then_branch,
                else_branch,
                ..
            } => self.generate_if(condition, then_branch, else_branch.as_deref()),

            ASTNode::LoopExpr {
                variable,
                init,
                condition,
                step,
                body,
                ..
            } => self.generate_loop(variable, init, condition, step, body),

            ASTNode::Lambda { .. } => {
                Err("lambdas must be lifted before IR generation".to_string())
            }

            ASTNode::FieldAccess { object, field, .. } => {
                let (obj_temp, _) = self.generate_expr(object)?;
                let ty = self.field_type(field);
                let result = self.new_temp();
                self.push_instruction(Instruction::GetField {
                    result: result.clone(),
                    object: obj_temp,
                    field: field.clone(),
                    ty: ty.clone(),
                });
                Ok((result, ty))
            }

            ASTNode::Index { array, index, .. } => {
                let (arr_temp, arr_ty) = self.generate_expr(array)?;
                let (idx_temp, _) = self.generate_expr(index)?;
                let elem_ty = elem_type_of(&arr_ty);
                let result = self.new_temp();
                self.push_instruction(Instruction::GetIndex {
                    result: result.clone(),
                    array: arr_temp,
                    index: idx_temp,
                    ty: elem_ty.clone(),
                });
                Ok((result, elem_ty))
            }

            ASTNode::AddrOf { operand, .. } => {
                let (temp, ty) = self.generate_expr(operand)?;
                let ptr_ty = format!("ptr_{ty}");
                let result = self.new_temp();
                self.push_instruction(Instruction::AddrOf {
                    result: result.clone(),
                    operand: temp,
                    ty: ptr_ty.clone(),
                });
                Ok((result, ptr_ty))
            }

            ASTNode::Deref { operand, .. } => {
                let (temp, ty) = self.generate_expr(operand)?;
                let pointee_ty = ty.strip_prefix("ptr_").unwrap_or("i64").to_string();
                let result = self.new_temp();
                self.push_instruction(Instruction::Deref {
                    result: result.clone(),
                    operand: temp,
                    ty: pointee_ty.clone(),
                });
                Ok((result, pointee_ty))
            }

            ASTNode::New {
                type_str,
                size_or_init,
                ..
            } => {
                let size = match size_or_init {
                    Some(init) => Some(self.generate_expr(init)?.0),
                    None => None,
                };
                let result = self.new_temp();
                self.push_instruction(Instruction::Alloc {
                    result: result.clone(),
                    ty: type_str.clone(),
                    size,
                });
                Ok((result, type_str.clone()))
            }

            ASTNode::ArrayLiteral { elements, .. } => {
                // No dedicated "array-of-values" literal instruction exists;
                // model it as allocating space for the array and writing
                // each element in with `get_index`'s write counterpart --
                // except there is no write counterpart either (this
                // language's arrays are read-only in the AST). Allocate and
                // return it; the element instructions above still run for
                // their (side-effect-free, in this MVP) value.
                let mut elem_ty = "i64".to_string();
                for element in elements {
                    let (_, ty) = self.generate_expr(element)?;
                    elem_ty = ty;
                }
                let arr_ty = format!("arr_{elem_ty}");
                let result = self.new_temp();
                self.push_instruction(Instruction::Alloc {
                    result: result.clone(),
                    ty: arr_ty.clone(),
                    size: None,
                });
                Ok((result, arr_ty))
            }

            ASTNode::Program { .. } | ASTNode::FunctionDef { .. } | ASTNode::StructDef { .. } => {
                Err(format!("'{node:?}' cannot appear inside an expression"))
            }
        }
    }

    /// `if` as an expression: branch, evaluate each side in its own block,
    /// then merge with a `Phi` in a join block. Works in tail and non-tail
    /// position alike -- the caller just keeps using the returned value in
    /// whatever block is current afterward.
    fn generate_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &ASTNode,
        else_branch: Option<&ASTNode>,
    ) -> Result<(String, String), String> {
        let (cond_temp, _) = self.generate_expr(condition)?;
        let then_label = self.new_label("if_then");
        let else_label = self.new_label("if_else");
        let merge_label = self.new_label("if_merge");

        self.push_instruction(Instruction::Branch {
            cond: cond_temp,
            true_label: then_label.clone(),
            false_label: else_label.clone(),
        });
        self.finish_block();

        self.current_label = then_label;
        let (then_value, then_ty) = self.generate_expr(then_branch)?;
        let then_end_label = self.current_label.clone();
        self.push_instruction(Instruction::Jump {
            label: merge_label.clone(),
        });
        self.finish_block();

        self.current_label = else_label;
        let (else_value, _) = match else_branch {
            Some(else_branch) => self.generate_expr(else_branch)?,
            None => {
                // No else branch: this project's test programs never hit
                // this (every `if` has one), so fall back to a default
                // value of the then-branch's type rather than modeling a
                // real "unit" type.
                let result = self.new_temp();
                self.push_instruction(Instruction::Const {
                    result: result.clone(),
                    value: default_value_for_type(&then_ty),
                    ty: then_ty.clone(),
                });
                (result, then_ty.clone())
            }
        };
        let else_end_label = self.current_label.clone();
        self.push_instruction(Instruction::Jump {
            label: merge_label.clone(),
        });
        self.finish_block();

        self.current_label = merge_label;
        let result = self.new_temp();
        self.push_instruction(Instruction::Phi {
            result: result.clone(),
            incoming: vec![(then_end_label, then_value), (else_end_label, else_value)],
            ty: then_ty.clone(),
        });
        Ok((result, then_ty))
    }

    /// `loop` as an expression: a header block's `Phi` carries the loop
    /// variable in from either the pre-loop block or the body's back-edge;
    /// the loop's value is the variable's value at the point the condition
    /// finally fails (the only value that naturally reaches the exit block).
    fn generate_loop(
        &mut self,
        variable: &str,
        init: &ASTNode,
        condition: &ASTNode,
        step: &ASTNode,
        body: &ASTNode,
    ) -> Result<(String, String), String> {
        let (init_temp, init_ty) = self.generate_expr(init)?;
        let before_label = self.current_label.clone();

        let header_label = self.new_label("loop_header");
        let body_label = self.new_label("loop_body");
        let exit_label = self.new_label("loop_exit");

        self.push_instruction(Instruction::Jump {
            label: header_label.clone(),
        });
        self.finish_block();

        self.current_label = header_label.clone();
        let var_temp = self.new_temp();
        // The back-edge from the loop body isn't known yet; patched in below
        // once it's been generated (standard SSA loop-header construction).
        self.push_instruction(Instruction::Phi {
            result: var_temp.clone(),
            incoming: vec![(before_label, init_temp)],
            ty: init_ty.clone(),
        });
        self.scopes.push(HashMap::new());
        self.declare(variable, var_temp.clone(), init_ty.clone());

        let (cond_temp, _) = self.generate_expr(condition)?;
        self.push_instruction(Instruction::Branch {
            cond: cond_temp,
            true_label: body_label.clone(),
            false_label: exit_label.clone(),
        });
        self.finish_block();

        self.current_label = body_label;
        let _ = self.generate_expr(body)?; // evaluated for side effects; the loop's value is the variable, not the body
        let (step_temp, _) = self.generate_expr(step)?;
        let latch_label = self.current_label.clone();
        self.push_instruction(Instruction::Jump {
            label: header_label.clone(),
        });
        self.finish_block();
        self.scopes.pop();

        self.patch_loop_phi(&header_label, &var_temp, latch_label, step_temp);

        self.current_label = exit_label;
        Ok((var_temp, init_ty))
    }

    /// Adds the loop body's back-edge to the header block's `Phi`, which was
    /// already flushed to `self.blocks` before the body (and thus the
    /// back-edge value) existed.
    fn patch_loop_phi(
        &mut self,
        header_label: &str,
        phi_result: &str,
        from_label: String,
        value: String,
    ) {
        let Some(block) = self.blocks.iter_mut().find(|b| b.label == header_label) else {
            return;
        };
        for instruction in &mut block.instructions {
            if let Instruction::Phi {
                result, incoming, ..
            } = instruction
                && result == phi_result
            {
                incoming.push((from_label, value));
                return;
            }
        }
    }

    fn generate_call(
        &mut self,
        function: &ASTNode,
        arguments: &[ASTNode],
    ) -> Result<(String, String), String> {
        if let ASTNode::Variable { name, .. } = function {
            if let Some(kind) = builtin_kind(name) {
                return self.generate_builtin_call(kind, name, arguments);
            }

            let mut arg_temps = vec![];
            for arg in arguments {
                let (temp, _) = self.generate_expr(arg)?;
                arg_temps.push(temp);
            }

            if self.functions.contains_key(name) {
                // A real top-level/lifted function: statically known name.
                let return_ty = match self.functions.get(name) {
                    Some(Type::Function { return_type, .. }) => return_type.mangled_name(),
                    Some(other) => other.mangled_name(),
                    None => "i64".to_string(),
                };
                let result = self.new_temp();
                self.push_instruction(Instruction::Call {
                    result: Some(result.clone()),
                    function: name.clone(),
                    arguments: arg_temps,
                    ty: Some(return_ty.clone()),
                });
                return Ok((result, return_ty));
            }

            // Not a known top-level function: this is a call through a
            // local variable/parameter holding a function value (e.g.
            // apply-func's `f`) -- a genuinely indirect call.
            let (function_value, ty) = self
                .lookup(name)
                .unwrap_or_else(|| (name.to_string(), "i64".to_string()));
            let return_ty = ty
                .strip_prefix("fn_")
                .and_then(|s| s.split("_to_").last())
                .unwrap_or("i64")
                .to_string();
            let result = self.new_temp();
            self.push_instruction(Instruction::CallIndirect {
                result: Some(result.clone()),
                function_value,
                arguments: arg_temps,
                ty: Some(return_ty.clone()),
            });
            return Ok((result, return_ty));
        }

        // Function position isn't a bare name (doesn't occur in this
        // grammar); whatever it evaluates to is necessarily a value, not a
        // static name, so this is indirect too.
        let (function_value, ty) = self.generate_expr(function)?;
        let mut arg_temps = vec![];
        for arg in arguments {
            let (temp, _) = self.generate_expr(arg)?;
            arg_temps.push(temp);
        }
        let result = self.new_temp();
        self.push_instruction(Instruction::CallIndirect {
            result: Some(result.clone()),
            function_value,
            arguments: arg_temps,
            ty: Some(ty.clone()),
        });
        Ok((result, ty))
    }

    fn generate_builtin_call(
        &mut self,
        kind: BuiltinKind,
        name: &str,
        arguments: &[ASTNode],
    ) -> Result<(String, String), String> {
        let mut arg_pairs = vec![];
        for arg in arguments {
            arg_pairs.push(self.generate_expr(arg)?);
        }

        match kind {
            BuiltinKind::Arith => self.generate_arith(name, arg_pairs),
            BuiltinKind::Cmp => self.generate_fold(name, &arg_pairs, "bool", true),
            BuiltinKind::Logical => self.generate_fold(name, &arg_pairs, "bool", false),
            BuiltinKind::Not => self.generate_not(arg_pairs),
            BuiltinKind::Print | BuiltinKind::Malloc | BuiltinKind::Free => {
                let arg_temps: Vec<String> = arg_pairs.iter().map(|(t, _)| t.clone()).collect();
                let ty = if kind == BuiltinKind::Malloc {
                    "ptr_i64".to_string()
                } else {
                    "i64".to_string()
                };
                let result = self.new_temp();
                self.push_instruction(Instruction::Call {
                    result: Some(result.clone()),
                    function: name.to_string(),
                    arguments: arg_temps,
                    ty: Some(ty.clone()),
                });
                Ok((result, ty))
            }
        }
    }

    fn generate_arith(
        &mut self,
        op: &str,
        arg_pairs: Vec<(String, String)>,
    ) -> Result<(String, String), String> {
        match arg_pairs.len() {
            0 => {
                let result = self.new_temp();
                self.push_instruction(Instruction::Const {
                    result: result.clone(),
                    value: serde_json::json!(0),
                    ty: "i64".into(),
                });
                Ok((result, "i64".to_string()))
            }
            1 => {
                let (temp, ty) = arg_pairs.into_iter().next().expect("checked len == 1");
                if op == "-" {
                    let zero = self.new_temp();
                    self.push_instruction(Instruction::Const {
                        result: zero.clone(),
                        value: serde_json::json!(0),
                        ty: ty.clone(),
                    });
                    let result = self.new_temp();
                    self.push_instruction(Instruction::BinOp {
                        result: result.clone(),
                        lhs: zero,
                        rhs: temp,
                        op_type: "-".to_string(),
                        ty: ty.clone(),
                    });
                    Ok((result, ty))
                } else {
                    Ok((temp, ty))
                }
            }
            _ => {
                let (mut acc_temp, mut acc_ty) = arg_pairs[0].clone();
                for (temp, ty) in &arg_pairs[1..] {
                    let result = self.new_temp();
                    self.push_instruction(Instruction::BinOp {
                        result: result.clone(),
                        lhs: acc_temp.clone(),
                        rhs: temp.clone(),
                        op_type: op.to_string(),
                        ty: acc_ty.clone(),
                    });
                    acc_temp = result;
                    acc_ty = ty.clone();
                }
                Ok((acc_temp, acc_ty))
            }
        }
    }

    /// Folds a chain of comparison/logical operands pairwise into `BinOp`s
    /// sharing `op`, defaulting to `default_value` when there are fewer
    /// than two operands.
    fn generate_fold(
        &mut self,
        op: &str,
        arg_pairs: &[(String, String)],
        result_ty: &str,
        default_value: bool,
    ) -> Result<(String, String), String> {
        if arg_pairs.len() < 2 {
            // Degenerate arity (0 or 1 operands): nothing to fold against,
            // so fall back to a default constant. Doesn't occur in this
            // project's test programs.
            let result = self.new_temp();
            self.push_instruction(Instruction::Const {
                result: result.clone(),
                value: serde_json::json!(default_value),
                ty: result_ty.to_string(),
            });
            return Ok((result, result_ty.to_string()));
        }

        let mut acc_temp = arg_pairs[0].0.clone();
        for (temp, _) in &arg_pairs[1..] {
            let result = self.new_temp();
            self.push_instruction(Instruction::BinOp {
                result: result.clone(),
                lhs: acc_temp.clone(),
                rhs: temp.clone(),
                op_type: op.to_string(),
                ty: result_ty.to_string(),
            });
            acc_temp = result;
        }
        Ok((acc_temp, result_ty.to_string()))
    }

    fn generate_not(
        &mut self,
        arg_pairs: Vec<(String, String)>,
    ) -> Result<(String, String), String> {
        let (temp, _) = arg_pairs
            .into_iter()
            .next()
            .ok_or_else(|| "`!` needs one operand".to_string())?;
        let false_temp = self.new_temp();
        self.push_instruction(Instruction::Const {
            result: false_temp.clone(),
            value: serde_json::json!(false),
            ty: "bool".into(),
        });
        let result = self.new_temp();
        self.push_instruction(Instruction::BinOp {
            result: result.clone(),
            lhs: temp,
            rhs: false_temp,
            op_type: "==".to_string(),
            ty: "bool".into(),
        });
        Ok((result, "bool".to_string()))
    }

    fn field_type(&self, field: &str) -> String {
        for fields in self.struct_fields.values() {
            if let Some((_, ty)) = fields.iter().find(|(name, _)| name == field) {
                return ty.mangled_name();
            }
        }
        "i64".to_string()
    }

    fn declare(&mut self, name: &str, temp: String, ty: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), (temp, ty));
        }
    }

    fn lookup(&self, name: &str) -> Option<(String, String)> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Some(binding.clone());
            }
        }
        None
    }

    fn new_temp(&mut self) -> String {
        let temp = format!("%v{}", self.temp_counter);
        self.temp_counter += 1;
        temp
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}_{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    fn push_instruction(&mut self, instruction: Instruction) {
        self.current_instructions.push(instruction);
    }

    fn finish_block(&mut self) {
        let label = self.current_label.clone();
        let instructions = std::mem::take(&mut self.current_instructions);
        self.blocks.push(BasicBlock {
            label,
            instructions,
        });
    }
}

fn elem_type_of(arr_ty: &str) -> String {
    arr_ty.strip_prefix("arr_").unwrap_or("i64").to_string()
}

fn literal_ir_type(literal_type: &str) -> String {
    match literal_type {
        "int64" => "i64".to_string(),
        "float64" => "f64".to_string(),
        "bool" => "bool".to_string(),
        "string" => "string".to_string(),
        other => other.to_string(),
    }
}

fn default_value_for_type(ty: &str) -> serde_json::Value {
    match ty {
        "i64" => serde_json::json!(0),
        "f64" => serde_json::json!(0.0),
        "bool" => serde_json::json!(false),
        "string" => serde_json::json!(""),
        _ => serde_json::Value::Null,
    }
}
