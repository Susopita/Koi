#[path = "support.rs"]
mod support;
use support::*;

use koi_ir::ir::{BasicBlock, IRFunction, IRProgram, Instruction};
use koi_ir::ir_generator::IRGenerator;
use koi_ir::types::Type;
use std::collections::{HashMap, HashSet};

fn generate(
    prog: &koi_ir::ast::ASTNode,
    functions: &HashMap<String, Type>,
    struct_fields: &HashMap<String, Vec<(String, Type)>>,
) -> IRProgram {
    IRGenerator::new(functions, struct_fields)
        .generate_program(prog)
        .expect("expected IR generation to succeed")
}

fn find_function<'a>(ir: &'a IRProgram, name: &str) -> &'a IRFunction {
    ir.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no function named '{name}' in {ir:?}"))
}

fn all_instructions(func: &IRFunction) -> Vec<&Instruction> {
    func.blocks
        .iter()
        .flat_map(|b: &BasicBlock| b.instructions.iter())
        .collect()
}

fn count(instrs: &[&Instruction], pred: impl Fn(&Instruction) -> bool) -> usize {
    instrs.iter().filter(|i| pred(i)).count()
}

fn fn_type(params: Vec<Type>, ret: Type) -> Type {
    Type::Function {
        params,
        return_type: Box::new(ret),
    }
}

#[test]
fn literal_emits_a_const_and_the_function_returns_it() {
    let prog = program(vec![defn("f", vec![], int(42))]);
    let functions = HashMap::from([("f".to_string(), fn_type(vec![], Type::Int64))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let f = find_function(&ir, "f");
    let instrs = all_instructions(f);
    assert!(
        matches!(instrs[0], Instruction::Const { value, ty, .. } if *value == serde_json::json!(42) && ty == "i64")
    );
    assert!(matches!(
        instrs.last().unwrap(),
        Instruction::Return { value: Some(_) }
    ));
}

#[test]
fn multi_arg_arithmetic_folds_into_a_chain_of_binops() {
    let prog = program(vec![defn(
        "f",
        vec![],
        call_named("+", vec![int(1), int(2), int(3)]),
    )]);
    let functions = HashMap::from([("f".to_string(), fn_type(vec![], Type::Int64))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "f"));
    assert_eq!(
        count(
            &instrs,
            |i| matches!(i, Instruction::BinOp { op_type, .. } if op_type == "+")
        ),
        2
    );
}

#[test]
fn comparison_emits_a_bool_typed_binop() {
    let prog = program(vec![defn(
        "f",
        vec![],
        call_named("<", vec![int(1), int(2)]),
    )]);
    let functions = HashMap::from([("f".to_string(), fn_type(vec![], Type::Bool))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "f"));
    assert!(instrs.iter().any(
        |i| matches!(i, Instruction::BinOp { op_type, ty, .. } if op_type == "<" && ty == "bool")
    ));
}

#[test]
fn logical_not_emits_an_equals_false_binop() {
    let prog = program(vec![defn(
        "f",
        vec![],
        call_named("!", vec![bool_lit(true)]),
    )]);
    let functions = HashMap::from([("f".to_string(), fn_type(vec![], Type::Bool))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "f"));
    assert!(instrs.iter().any(
        |i| matches!(i, Instruction::Const { value, .. } if *value == serde_json::json!(false))
    ));
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::BinOp { op_type, .. } if op_type == "=="))
    );
}

#[test]
fn calling_a_known_top_level_function_emits_call() {
    let prog = program(vec![defn(
        "main",
        vec![],
        call_named("add", vec![int(1), int(2)]),
    )]);
    let functions = HashMap::from([
        ("main".to_string(), fn_type(vec![], Type::Int64)),
        (
            "add".to_string(),
            fn_type(vec![Type::Int64, Type::Int64], Type::Int64),
        ),
    ]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "main"));
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::Call { function, .. } if function == "add"))
    );
    assert!(
        !instrs
            .iter()
            .any(|i| matches!(i, Instruction::CallIndirect { .. }))
    );
}

#[test]
fn calling_a_function_valued_parameter_emits_call_indirect() {
    let prog = program(vec![defn(
        "apply-func",
        vec![("f", None), ("x", None)],
        call_named("f", vec![var("x")]),
    )]);
    let functions = HashMap::from([(
        "apply-func".to_string(),
        fn_type(
            vec![fn_type(vec![Type::Int64], Type::Int64), Type::Int64],
            Type::Int64,
        ),
    )]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "apply-func"));
    assert!(instrs.iter().any(
        |i| matches!(i, Instruction::CallIndirect { function_value, .. } if function_value == "f")
    ));
    assert!(!instrs.iter().any(|i| matches!(i, Instruction::Call { .. })));
}

#[test]
fn if_expression_branches_and_merges_with_a_phi() {
    let prog = program(vec![defn(
        "f",
        vec![("x", None)],
        if_expr(var("x"), int(1), Some(int(2))),
    )]);
    let functions = HashMap::from([("f".to_string(), fn_type(vec![Type::Bool], Type::Int64))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let f = find_function(&ir, "f");
    assert_eq!(
        f.blocks.len(),
        4,
        "expected entry/then/else/merge blocks, got: {f:?}"
    );

    let entry = &f.blocks[0];
    assert!(matches!(
        entry.instructions.last().unwrap(),
        Instruction::Branch { .. }
    ));

    let merge = f.blocks.last().unwrap();
    let phi = merge
        .instructions
        .iter()
        .find(|i| matches!(i, Instruction::Phi { .. }))
        .expect("expected a phi in the merge block");
    let Instruction::Phi { incoming, .. } = phi else {
        unreachable!()
    };
    assert_eq!(incoming.len(), 2);
    assert!(matches!(
        merge.instructions.last().unwrap(),
        Instruction::Return { .. }
    ));
}

#[test]
fn loop_header_phi_gets_its_back_edge_patched() {
    let prog = program(vec![defn(
        "f",
        vec![("n", None)],
        loop_expr(
            "i",
            int(0),
            call_named("<", vec![var("i"), var("n")]),
            call_named("+", vec![var("i"), int(1)]),
            call_named("+", vec![var("i"), int(1)]),
        ),
    )]);
    let functions = HashMap::from([("f".to_string(), fn_type(vec![Type::Int64], Type::Int64))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let f = find_function(&ir, "f");
    let header = f
        .blocks
        .iter()
        .find(|b| b.label.starts_with("loop_header"))
        .expect("expected a loop header block");
    let phi = header
        .instructions
        .iter()
        .find(|i| matches!(i, Instruction::Phi { .. }))
        .expect("expected a phi in the loop header");
    let Instruction::Phi { incoming, .. } = phi else {
        unreachable!()
    };
    assert_eq!(
        incoming.len(),
        2,
        "expected both the pre-loop edge and the patched back-edge"
    );

    let body = f
        .blocks
        .iter()
        .find(|b| b.label.starts_with("loop_body"))
        .expect("expected a loop body block");
    assert!(
        matches!(body.instructions.last().unwrap(), Instruction::Jump { label } if label.starts_with("loop_header"))
    );
}

#[test]
fn field_access_emits_get_field() {
    let prog = program(vec![defn(
        "get-x",
        vec![("p", None)],
        field_access(var("p"), "x"),
    )]);
    let functions = HashMap::from([(
        "get-x".to_string(),
        fn_type(vec![Type::Struct("Point".to_string())], Type::Int64),
    )]);
    let struct_fields = HashMap::from([(
        "Point".to_string(),
        vec![
            ("x".to_string(), Type::Int64),
            ("y".to_string(), Type::Int64),
        ],
    )]);
    let ir = generate(&prog, &functions, &struct_fields);

    let instrs = all_instructions(find_function(&ir, "get-x"));
    assert!(instrs.iter().any(
        |i| matches!(i, Instruction::GetField { field, ty, .. } if field == "x" && ty == "i64")
    ));
}

#[test]
fn index_emits_get_index() {
    let prog = program(vec![defn(
        "f",
        vec![],
        index(array_literal(vec![int(1), int(2)]), int(0)),
    )]);
    let functions = HashMap::from([("f".to_string(), fn_type(vec![], Type::Int64))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "f"));
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::Alloc { .. }))
    );
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::GetIndex { .. }))
    );
}

#[test]
fn addr_of_and_deref_emit_dedicated_instructions() {
    let prog = program(vec![defn("f", vec![("x", None)], deref(addr_of(var("x"))))]);
    let functions = HashMap::from([("f".to_string(), fn_type(vec![Type::Int64], Type::Int64))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "f"));
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::AddrOf { .. }))
    );
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::Deref { .. }))
    );
}

#[test]
fn new_emits_alloc() {
    let prog = program(vec![defn("f", vec![], new_expr("Point", None))]);
    let functions = HashMap::from([(
        "f".to_string(),
        fn_type(vec![], Type::Struct("Point".to_string())),
    )]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "f"));
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::Alloc { ty, .. } if ty == "Point"))
    );
}

#[test]
fn every_function_ends_in_a_return() {
    let prog = program(vec![
        defn(
            "f",
            vec![("x", None)],
            if_expr(var("x"), int(1), Some(int(2))),
        ),
        defn("g", vec![], int(1)),
    ]);
    let functions = HashMap::from([
        ("f".to_string(), fn_type(vec![Type::Bool], Type::Int64)),
        ("g".to_string(), fn_type(vec![], Type::Int64)),
    ]);
    let ir = generate(&prog, &functions, &HashMap::new());

    for f in &ir.functions {
        let last_block = f.blocks.last().unwrap();
        assert!(
            matches!(
                last_block.instructions.last().unwrap(),
                Instruction::Return { .. }
            ),
            "function {} doesn't end in return",
            f.name
        );
    }
}

#[test]
fn ssa_results_are_never_assigned_twice() {
    let prog = program(vec![defn(
        "fib",
        vec![("n", None)],
        if_expr(
            call_named("<=", vec![var("n"), int(1)]),
            var("n"),
            Some(call_named(
                "+",
                vec![
                    call_named("fib", vec![call_named("-", vec![var("n"), int(1)])]),
                    call_named("fib", vec![call_named("-", vec![var("n"), int(2)])]),
                ],
            )),
        ),
    )]);
    let functions = HashMap::from([("fib".to_string(), fn_type(vec![Type::Int64], Type::Int64))]);
    let ir = generate(&prog, &functions, &HashMap::new());

    let instrs = all_instructions(find_function(&ir, "fib"));
    let results: Vec<&str> = instrs
        .iter()
        .filter_map(|i| match i {
            Instruction::Const { result, .. }
            | Instruction::BinOp { result, .. }
            | Instruction::Phi { result, .. }
            | Instruction::Alloc { result, .. }
            | Instruction::GetField { result, .. }
            | Instruction::GetIndex { result, .. }
            | Instruction::AddrOf { result, .. }
            | Instruction::Deref { result, .. } => Some(result.as_str()),
            Instruction::Call {
                result: Some(r), ..
            }
            | Instruction::CallIndirect {
                result: Some(r), ..
            } => Some(r.as_str()),
            _ => None,
        })
        .collect();

    let unique: HashSet<&str> = results.iter().copied().collect();
    assert_eq!(
        results.len(),
        unique.len(),
        "an SSA temp was assigned more than once: {results:?}"
    );
}
