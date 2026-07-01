use std::fs;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

const BIN: &str = env!("CARGO_BIN_EXE_koi-ir");
const AST_JSON: &str = "/tmp/ast.json";
const IR_JSON: &str = "/tmp/ir.json";

/// koi-ir always reads from the fixed path `/tmp/ast.json` and writes to
/// `/tmp/ir.json` -- every test that touches either file needs to run
/// exclusive of every other such test (cargo's default harness runs tests
/// from one binary concurrently on separate threads). See koi-ast's
/// cli_tests.rs for the same convention.
fn shared_files_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

const VALID_ADD_AST: &str = r#"
{
  "nodeType": "program",
  "children": [
    {
      "nodeType": "function_def",
      "name": "add",
      "parameters": [["x", null], ["y", null]],
      "body": {
        "nodeType": "call",
        "function": {"nodeType": "variable", "name": "+", "line": 2, "column": 4},
        "arguments": [
          {"nodeType": "variable", "name": "x", "line": 2, "column": 6},
          {"nodeType": "variable", "name": "y", "line": 2, "column": 8}
        ],
        "line": 2, "column": 3
      },
      "line": 1, "column": 1
    }
  ]
}
"#;

#[test]
fn valid_ast_json_exits_zero_and_writes_schema_valid_ir() {
    let _guard = shared_files_lock().lock().unwrap();
    fs::write(AST_JSON, VALID_ADD_AST).unwrap();
    let _ = fs::remove_file(IR_JSON);

    let output = Command::new(BIN).output().expect("failed to run koi-ir");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("IR complete"));

    let json = fs::read_to_string(IR_JSON).expect("koi-ir should have written /tmp/ir.json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("output must be valid JSON");
    assert_eq!(value["irType"], "hir");
    assert_eq!(value["functions"][0]["name"], "add");
}

#[test]
fn missing_ast_json_reports_inference_prefixed_error() {
    let _guard = shared_files_lock().lock().unwrap();
    let _ = fs::remove_file(AST_JSON);

    let output = Command::new(BIN).output().expect("failed to run koi-ir");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("[inference]"));
}

#[test]
fn malformed_ast_json_reports_inference_prefixed_error() {
    let _guard = shared_files_lock().lock().unwrap();
    fs::write(AST_JSON, "{ this is not valid json").unwrap();

    let output = Command::new(BIN).output().expect("failed to run koi-ir");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("[inference]"));
}

#[test]
fn undeclared_variable_reports_inference_prefixed_error_and_exits_nonzero() {
    let _guard = shared_files_lock().lock().unwrap();
    let ast = r#"
    {
      "nodeType": "program",
      "children": [
        {
          "nodeType": "function_def",
          "name": "f",
          "parameters": [],
          "body": {"nodeType": "variable", "name": "z", "line": 1, "column": 1},
          "line": 1, "column": 1
        }
      ]
    }
    "#;
    fs::write(AST_JSON, ast).unwrap();

    let output = Command::new(BIN).output().expect("failed to run koi-ir");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("[inference]"));
}

#[test]
fn type_mismatch_reports_unification_prefixed_error_and_exits_nonzero() {
    let _guard = shared_files_lock().lock().unwrap();
    // `f`'s body mixes an int64 and a bool literal in one array -- arrays
    // must be homogeneous.
    let ast = r#"
    {
      "nodeType": "program",
      "children": [
        {
          "nodeType": "function_def",
          "name": "f",
          "parameters": [],
          "body": {
            "nodeType": "array_literal",
            "elements": [
              {"nodeType": "literal", "literalType": "int64", "value": 1, "line": 1, "column": 1},
              {"nodeType": "literal", "literalType": "bool", "value": true, "line": 1, "column": 1}
            ],
            "line": 1, "column": 1
          },
          "line": 1, "column": 1
        }
      ]
    }
    "#;
    fs::write(AST_JSON, ast).unwrap();

    let output = Command::new(BIN).output().expect("failed to run koi-ir");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("[unification]"));
}
