use crate::ast::ASTNode;
use crate::builtins::BUILTIN_NAMES;
use std::collections::HashSet;

pub struct LambdaLifter {
    lifted_functions: Vec<ASTNode>,
    lambda_counter: usize,
    globals: HashSet<String>,
}

impl LambdaLifter {
    pub fn new(top_level_function_names: HashSet<String>) -> Self {
        let mut globals = top_level_function_names;
        globals.extend(BUILTIN_NAMES.iter().map(|s| s.to_string()));
        LambdaLifter {
            lifted_functions: vec![],
            lambda_counter: 0,
            globals,
        }
    }

    /// Scans `program` for top-level function names to seed `globals`
    /// automatically.
    pub fn for_program(program: &ASTNode) -> Self {
        let mut names = HashSet::new();
        if let ASTNode::Program { children } = program {
            for child in children {
                if let ASTNode::FunctionDef { name, .. } = child {
                    names.insert(name.clone());
                }
            }
        }
        Self::new(names)
    }

    /// Lifts every lambda in `program` to a top-level function, returning a
    /// new program with the lifted functions prepended.
    pub fn lift_program(&mut self, program: &ASTNode) -> ASTNode {
        let rewritten = self.lift_node(program);
        match rewritten {
            ASTNode::Program { children } => {
                let mut all = std::mem::take(&mut self.lifted_functions);
                all.extend(children);
                ASTNode::Program { children: all }
            }
            other => other,
        }
    }

    fn lift_node(&mut self, node: &ASTNode) -> ASTNode {
        match node {
            ASTNode::Program { children } => ASTNode::Program {
                children: children.iter().map(|c| self.lift_node(c)).collect(),
            },
            ASTNode::FunctionDef {
                name,
                parameters,
                body,
                line,
                column,
            } => ASTNode::FunctionDef {
                name: name.clone(),
                parameters: parameters.clone(),
                body: Box::new(self.lift_node(body)),
                line: *line,
                column: *column,
            },
            ASTNode::StructDef { .. } | ASTNode::Variable { .. } | ASTNode::Literal { .. } => {
                node.clone()
            }

            ASTNode::Lambda {
                parameters,
                body,
                line,
                column,
            } => self.lift_lambda(parameters, body, *line, *column),

            ASTNode::Call {
                function,
                arguments,
                line,
                column,
            } => ASTNode::Call {
                function: Box::new(self.lift_node(function)),
                arguments: arguments.iter().map(|a| self.lift_node(a)).collect(),
                line: *line,
                column: *column,
            },
            ASTNode::LetBinding {
                bindings,
                body,
                line,
                column,
            } => ASTNode::LetBinding {
                bindings: bindings
                    .iter()
                    .map(|(n, v)| (n.clone(), Box::new(self.lift_node(v))))
                    .collect(),
                body: Box::new(self.lift_node(body)),
                line: *line,
                column: *column,
            },
            ASTNode::IfExpr {
                condition,
                then_branch,
                else_branch,
                line,
                column,
            } => ASTNode::IfExpr {
                condition: Box::new(self.lift_node(condition)),
                then_branch: Box::new(self.lift_node(then_branch)),
                else_branch: else_branch.as_ref().map(|e| Box::new(self.lift_node(e))),
                line: *line,
                column: *column,
            },
            ASTNode::LoopExpr {
                variable,
                init,
                condition,
                step,
                body,
                line,
                column,
            } => ASTNode::LoopExpr {
                variable: variable.clone(),
                init: Box::new(self.lift_node(init)),
                condition: Box::new(self.lift_node(condition)),
                step: Box::new(self.lift_node(step)),
                body: Box::new(self.lift_node(body)),
                line: *line,
                column: *column,
            },
            ASTNode::FieldAccess {
                object,
                field,
                line,
                column,
            } => ASTNode::FieldAccess {
                object: Box::new(self.lift_node(object)),
                field: field.clone(),
                line: *line,
                column: *column,
            },
            ASTNode::Index {
                array,
                index,
                line,
                column,
            } => ASTNode::Index {
                array: Box::new(self.lift_node(array)),
                index: Box::new(self.lift_node(index)),
                line: *line,
                column: *column,
            },
            ASTNode::AddrOf {
                operand,
                line,
                column,
            } => ASTNode::AddrOf {
                operand: Box::new(self.lift_node(operand)),
                line: *line,
                column: *column,
            },
            ASTNode::Deref {
                operand,
                line,
                column,
            } => ASTNode::Deref {
                operand: Box::new(self.lift_node(operand)),
                line: *line,
                column: *column,
            },
            ASTNode::New {
                type_str,
                size_or_init,
                line,
                column,
            } => ASTNode::New {
                type_str: type_str.clone(),
                size_or_init: size_or_init.as_ref().map(|e| Box::new(self.lift_node(e))),
                line: *line,
                column: *column,
            },
            ASTNode::ArrayLiteral {
                elements,
                line,
                column,
            } => ASTNode::ArrayLiteral {
                elements: elements.iter().map(|e| self.lift_node(e)).collect(),
                line: *line,
                column: *column,
            },
        }
    }

    fn lift_lambda(
        &mut self,
        parameters: &[(String, Option<String>)],
        body: &ASTNode,
        line: usize,
        column: usize,
    ) -> ASTNode {
        // Lift any nested lambdas first, so their own captures are already
        // resolved to plain Variable references (or closure-construction
        // calls) by the time we look for *this* lambda's free variables.
        let lifted_body = self.lift_node(body);

        let bound: HashSet<String> = parameters.iter().map(|(n, _)| n.clone()).collect();
        let mut free_vars = HashSet::new();
        collect_free_variables(&lifted_body, &bound, &self.globals, &mut free_vars);

        let id = self.lambda_counter;
        self.lambda_counter += 1;
        let func_name = format!("_lambda_{id}");

        // Register this lambda's lifted name (and its closure-constructor
        // placeholder, used below in the captures path) as globals *before*
        // any enclosing lambda gets to analyze its own free variables --
        // otherwise a reference to this lambda's lifted name/constructor,
        // appearing in an *outer* lambda's body, would be mistaken for a
        // captured variable instead of a reference to a global function.
        self.globals.insert(func_name.clone());
        self.globals.insert(format!("__make_closure_{func_name}"));

        if free_vars.is_empty() {
            self.lifted_functions.push(ASTNode::FunctionDef {
                name: func_name.clone(),
                parameters: parameters.to_vec(),
                body: Box::new(lifted_body),
                line,
                column,
            });
            // No captures -- the lifted function's name stands in directly
            // for the lambda value.
            return ASTNode::Variable {
                name: func_name,
                line,
                column,
            };
        }

        // Captures path: no test program currently exercises this. Captured
        // fields default to i64 (this MVP's fallback numeric type) since
        // there's no typed-AST plumbing at this stage to know their real
        // types; the closure "construction" below is a placeholder call
        // rather than a real allocation, since the AST has no struct-literal
        // node. Nothing downstream (koi-assembly doesn't exist yet) consumes
        // this, so it's documented as unverified rather than fully solved.
        let env_struct_name = format!("_Lambda_{id}_Env");
        let mut captured: Vec<String> = free_vars.iter().cloned().collect();
        captured.sort();

        self.lifted_functions.push(ASTNode::StructDef {
            name: env_struct_name.clone(),
            fields: captured
                .iter()
                .map(|v| (v.clone(), "i64".to_string()))
                .collect(),
            line,
            column,
        });

        let mut lifted_params = vec![("env".to_string(), Some(env_struct_name))];
        lifted_params.extend(parameters.iter().cloned());

        let rewritten_body = rewrite_free_var_access(&lifted_body, &free_vars);

        self.lifted_functions.push(ASTNode::FunctionDef {
            name: func_name.clone(),
            parameters: lifted_params,
            body: Box::new(rewritten_body),
            line,
            column,
        });

        ASTNode::Call {
            function: Box::new(ASTNode::Variable {
                name: format!("__make_closure_{func_name}"),
                line,
                column,
            }),
            arguments: captured
                .into_iter()
                .map(|v| ASTNode::Variable {
                    name: v,
                    line,
                    column,
                })
                .collect(),
            line,
            column,
        }
    }
}

fn collect_free_variables(
    node: &ASTNode,
    bound: &HashSet<String>,
    globals: &HashSet<String>,
    free: &mut HashSet<String>,
) {
    match node {
        ASTNode::Variable { name, .. } => {
            if !bound.contains(name) && !globals.contains(name) {
                free.insert(name.clone());
            }
        }
        ASTNode::Literal { .. } => {}
        ASTNode::Call {
            function,
            arguments,
            ..
        } => {
            collect_free_variables(function, bound, globals, free);
            for arg in arguments {
                collect_free_variables(arg, bound, globals, free);
            }
        }
        ASTNode::LetBinding { bindings, body, .. } => {
            let mut inner_bound = bound.clone();
            for (name, value) in bindings {
                collect_free_variables(value, &inner_bound, globals, free);
                inner_bound.insert(name.clone());
            }
            collect_free_variables(body, &inner_bound, globals, free);
        }
        ASTNode::IfExpr {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_free_variables(condition, bound, globals, free);
            collect_free_variables(then_branch, bound, globals, free);
            if let Some(e) = else_branch {
                collect_free_variables(e, bound, globals, free);
            }
        }
        ASTNode::LoopExpr {
            variable,
            init,
            condition,
            step,
            body,
            ..
        } => {
            collect_free_variables(init, bound, globals, free);
            let mut inner_bound = bound.clone();
            inner_bound.insert(variable.clone());
            collect_free_variables(condition, &inner_bound, globals, free);
            collect_free_variables(step, &inner_bound, globals, free);
            collect_free_variables(body, &inner_bound, globals, free);
        }
        ASTNode::Lambda {
            parameters, body, ..
        } => {
            let mut inner_bound = bound.clone();
            for (name, _) in parameters {
                inner_bound.insert(name.clone());
            }
            collect_free_variables(body, &inner_bound, globals, free);
        }
        ASTNode::FieldAccess { object, .. } => collect_free_variables(object, bound, globals, free),
        ASTNode::Index { array, index, .. } => {
            collect_free_variables(array, bound, globals, free);
            collect_free_variables(index, bound, globals, free);
        }
        ASTNode::AddrOf { operand, .. } | ASTNode::Deref { operand, .. } => {
            collect_free_variables(operand, bound, globals, free);
        }
        ASTNode::New { size_or_init, .. } => {
            if let Some(e) = size_or_init {
                collect_free_variables(e, bound, globals, free);
            }
        }
        ASTNode::ArrayLiteral { elements, .. } => {
            for e in elements {
                collect_free_variables(e, bound, globals, free);
            }
        }
        ASTNode::Program { .. } | ASTNode::FunctionDef { .. } | ASTNode::StructDef { .. } => {}
    }
}

fn rewrite_free_var_access(node: &ASTNode, free_vars: &HashSet<String>) -> ASTNode {
    match node {
        ASTNode::Variable { name, line, column } => {
            if free_vars.contains(name) {
                ASTNode::FieldAccess {
                    object: Box::new(ASTNode::Variable {
                        name: "env".to_string(),
                        line: *line,
                        column: *column,
                    }),
                    field: name.clone(),
                    line: *line,
                    column: *column,
                }
            } else {
                node.clone()
            }
        }
        ASTNode::Literal { .. }
        | ASTNode::StructDef { .. }
        | ASTNode::Program { .. }
        | ASTNode::FunctionDef { .. } => node.clone(),
        ASTNode::Call {
            function,
            arguments,
            line,
            column,
        } => ASTNode::Call {
            function: Box::new(rewrite_free_var_access(function, free_vars)),
            arguments: arguments
                .iter()
                .map(|a| rewrite_free_var_access(a, free_vars))
                .collect(),
            line: *line,
            column: *column,
        },
        ASTNode::LetBinding {
            bindings,
            body,
            line,
            column,
        } => {
            // A let-bound name shadows a captured free variable of the same
            // name for the rest of the let.
            let mut still_free = free_vars.clone();
            let mut new_bindings = vec![];
            for (name, value) in bindings {
                new_bindings.push((
                    name.clone(),
                    Box::new(rewrite_free_var_access(value, &still_free)),
                ));
                still_free.remove(name);
            }
            ASTNode::LetBinding {
                bindings: new_bindings,
                body: Box::new(rewrite_free_var_access(body, &still_free)),
                line: *line,
                column: *column,
            }
        }
        ASTNode::IfExpr {
            condition,
            then_branch,
            else_branch,
            line,
            column,
        } => ASTNode::IfExpr {
            condition: Box::new(rewrite_free_var_access(condition, free_vars)),
            then_branch: Box::new(rewrite_free_var_access(then_branch, free_vars)),
            else_branch: else_branch
                .as_ref()
                .map(|e| Box::new(rewrite_free_var_access(e, free_vars))),
            line: *line,
            column: *column,
        },
        ASTNode::LoopExpr {
            variable,
            init,
            condition,
            step,
            body,
            line,
            column,
        } => {
            let mut inner_free = free_vars.clone();
            inner_free.remove(variable);
            ASTNode::LoopExpr {
                variable: variable.clone(),
                init: Box::new(rewrite_free_var_access(init, free_vars)),
                condition: Box::new(rewrite_free_var_access(condition, &inner_free)),
                step: Box::new(rewrite_free_var_access(step, &inner_free)),
                body: Box::new(rewrite_free_var_access(body, &inner_free)),
                line: *line,
                column: *column,
            }
        }
        ASTNode::Lambda {
            parameters,
            body,
            line,
            column,
        } => {
            let mut inner_free = free_vars.clone();
            for (name, _) in parameters {
                inner_free.remove(name);
            }
            ASTNode::Lambda {
                parameters: parameters.clone(),
                body: Box::new(rewrite_free_var_access(body, &inner_free)),
                line: *line,
                column: *column,
            }
        }
        ASTNode::FieldAccess {
            object,
            field,
            line,
            column,
        } => ASTNode::FieldAccess {
            object: Box::new(rewrite_free_var_access(object, free_vars)),
            field: field.clone(),
            line: *line,
            column: *column,
        },
        ASTNode::Index {
            array,
            index,
            line,
            column,
        } => ASTNode::Index {
            array: Box::new(rewrite_free_var_access(array, free_vars)),
            index: Box::new(rewrite_free_var_access(index, free_vars)),
            line: *line,
            column: *column,
        },
        ASTNode::AddrOf {
            operand,
            line,
            column,
        } => ASTNode::AddrOf {
            operand: Box::new(rewrite_free_var_access(operand, free_vars)),
            line: *line,
            column: *column,
        },
        ASTNode::Deref {
            operand,
            line,
            column,
        } => ASTNode::Deref {
            operand: Box::new(rewrite_free_var_access(operand, free_vars)),
            line: *line,
            column: *column,
        },
        ASTNode::New {
            type_str,
            size_or_init,
            line,
            column,
        } => ASTNode::New {
            type_str: type_str.clone(),
            size_or_init: size_or_init
                .as_ref()
                .map(|e| Box::new(rewrite_free_var_access(e, free_vars))),
            line: *line,
            column: *column,
        },
        ASTNode::ArrayLiteral {
            elements,
            line,
            column,
        } => ASTNode::ArrayLiteral {
            elements: elements
                .iter()
                .map(|e| rewrite_free_var_access(e, free_vars))
                .collect(),
            line: *line,
            column: *column,
        },
    }
}
