use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

const AST_JSON: &str = "/tmp/ast.json";
const IR_JSON: &str = "/tmp/ir.json";
const ASM_PATH: &str = "output.s";

fn shared_pipeline_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_pipeline() -> std::sync::MutexGuard<'static, ()> {
    match shared_pipeline_lock().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

fn c_compiler() -> String {
    if let Ok(cc) = std::env::var("KOI_CC") {
        return cc;
    }
    if let Ok(cc) = std::env::var("CC") {
        return cc;
    }

    let zig = "/home/aleu/snap/codex/34/zig-x86_64-linux-0.16.0/zig";
    if Path::new(zig).is_file() {
        return zig.to_string();
    }

    for candidate in ["cc", "gcc", "clang"] {
        if Command::new(candidate).arg("--version").output().is_ok() {
            return candidate.to_string();
        }
    }

    panic!("no C compiler found; set KOI_CC or CC");
}

fn build_release_binaries() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let status = Command::new(cargo_bin())
            .args(["build", "--release", "-p", "koi-ast", "-p", "koi-ir", "-p", "koi-assembly"])
            .current_dir(workspace_root())
            .status()
            .expect("failed to run cargo build --release");
        assert!(status.success(), "release build failed");
    });
}

fn run_pipeline(sample: &str) -> PathBuf {
    let root = workspace_root();
    let sample_path = root.join("test/casos_prueba_carp").join(sample);
    let ast = root.join("target/release/koi-ast");
    let ir = root.join("target/release/koi-ir");
    let assembly = root.join("target/release/koi-assembly");

    let _ = fs::remove_file(AST_JSON);
    let _ = fs::remove_file(IR_JSON);
    let _ = fs::remove_file(root.join(ASM_PATH));

    let ast_output = Command::new(ast)
        .arg(&sample_path)
        .current_dir(&root)
        .output()
        .expect("failed to run koi-ast");
    assert!(
        ast_output.status.success(),
        "koi-ast failed for {sample}: {}",
        String::from_utf8_lossy(&ast_output.stderr)
    );

    let ir_output = Command::new(ir)
        .current_dir(&root)
        .output()
        .expect("failed to run koi-ir");
    assert!(
        ir_output.status.success(),
        "koi-ir failed for {sample}: {}",
        String::from_utf8_lossy(&ir_output.stderr)
    );

    let assembly_output = Command::new(assembly)
        .current_dir(&root)
        .output()
        .expect("failed to run koi-assembly");
    assert!(
        assembly_output.status.success(),
        "koi-assembly failed for {sample}: {}",
        String::from_utf8_lossy(&assembly_output.stderr)
    );

    let asm_path = root.join(ASM_PATH);
    assert!(asm_path.is_file(), "expected output.s to be generated");
    asm_path
}

fn assemble_and_link(asm_path: &Path, exe_name: &str) -> PathBuf {
    let root = workspace_root();
    let compiler = c_compiler();
    let obj_path = std::env::temp_dir().join(format!("{exe_name}.o"));
    let exe_path = std::env::temp_dir().join(exe_name);

    let mut compile = Command::new(&compiler);
    if compiler.ends_with("/zig") || compiler == "zig" {
        compile.arg("cc");
    }
    let compile_status = compile
        .arg("-c")
        .arg(asm_path)
        .arg("-o")
        .arg(&obj_path)
        .current_dir(&root)
        .status()
        .expect("failed to assemble output.s");
    assert!(compile_status.success(), "assembly failed for {:?}", asm_path);

    let mut link = Command::new(&compiler);
    if compiler.ends_with("/zig") || compiler == "zig" {
        link.arg("cc");
    }
    let link_status = link
        .arg(&obj_path)
        .arg("-o")
        .arg(&exe_path)
        .current_dir(&root)
        .status()
        .expect("failed to link executable");
    assert!(link_status.success(), "link failed for {:?}", asm_path);

    let _ = fs::remove_file(obj_path);
    exe_path
}

#[test]
fn add_program_runs_and_returns_expected_exit_code() {
    let _guard = lock_pipeline();
    build_release_binaries();

    let asm_path = run_pipeline("add.carp");
    let exe = assemble_and_link(&asm_path, "koi-pipeline-add");
    let output = Command::new(&exe)
        .output()
        .expect("failed to run linked add executable");
    assert_eq!(output.status.code(), Some(8));

    let _ = fs::remove_file(asm_path);
    let _ = fs::remove_file(exe);
}

#[test]
fn lambda_program_runs_and_returns_expected_exit_code() {
    let _guard = lock_pipeline();
    build_release_binaries();

    let asm_path = run_pipeline("lambda.carp");
    let exe = assemble_and_link(&asm_path, "koi-pipeline-lambda");
    let output = Command::new(&exe)
        .output()
        .expect("failed to run linked lambda executable");
    assert_eq!(output.status.code(), Some(6));

    let _ = fs::remove_file(asm_path);
    let _ = fs::remove_file(exe);
}

#[test]
fn control_flow_struct_and_kitchen_sink_reach_assembly_stage() {
    let _guard = lock_pipeline();
    build_release_binaries();

    for sample in ["control_flow.carp", "struct.carp", "kitchen_sink.carp"] {
        let asm_path = run_pipeline(sample);
        let compiler = c_compiler();
        let obj_path = std::env::temp_dir().join(format!("{sample}.o"));

        let mut compile = Command::new(&compiler);
        if compiler.ends_with("/zig") || compiler == "zig" {
            compile.arg("cc");
        }
        let status = compile
            .arg("-c")
            .arg(&asm_path)
            .arg("-o")
            .arg(&obj_path)
            .current_dir(workspace_root())
            .status()
            .expect("failed to assemble output.s");
        assert!(status.success(), "assembly failed for {sample}");

        let _ = fs::remove_file(&asm_path);
        let _ = fs::remove_file(obj_path);
    }
}
