use koi_ir::ast::ASTNode;
use koi_ir::pipeline;
use std::fs;

fn main() {
    let input = match fs::read_to_string("/tmp/ast.json") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("[inference] Could not read /tmp/ast.json: {e}");
            std::process::exit(1);
        }
    };

    let program: ASTNode = match serde_json::from_str(&input) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("[inference] /tmp/ast.json is not a valid AST: {e}");
            std::process::exit(1);
        }
    };

    let ir_program = match pipeline::compile(&program) {
        Ok(ir) => ir,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let json = match serde_json::to_string_pretty(&ir_program) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("[ir_generator] Failed to serialize IR: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = fs::write("/tmp/ir.json", json) {
        eprintln!("[ir_generator] Failed to write /tmp/ir.json: {e}");
        std::process::exit(1);
    }

    println!("IR complete. Saved to /tmp/ir.json");
}
