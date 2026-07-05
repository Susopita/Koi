# Tests

## Resumen

**322 tests, 0 failures** (8 ignorados en ARM64).

## Suites

### Lib unitarios — 102 tests

Estos tests se ejecutan como parte de `cargo test --lib` y cubren todos los módulos internos:

| Suite | Tests | Archivo(s) |
|---|---|---|
| `optimizer` | 13 | `src/backend/optimizer.rs` (inline) |
| `peephole` | 12 | `src/backend/x86_64/peephole.rs` (inline) |
| `register_allocator` | 4 | `src/backend/x86_64/register_allocator.rs` (inline) |
| `sexpr` (Reader) | 18 | `src/frontend/sexpr.rs` |
| `macro_expander` | 14 | `src/frontend/macro_expander.rs` |
| `typed_ast` | 5 | `src/frontend/typed_ast.rs` |
| `borrow_checker` | 5 | `src/frontend/borrow_checker.rs` |
| ARM64 scheduler | 4 | `src/backend/arm64/scheduler.rs` |
| ARM64 materializer | 4 | `src/backend/arm64/materializer.rs` |
| ARM64 register_allocator | 4 | `src/backend/arm64/register_allocator.rs` |
| RISC-V instruction_select | 5 | `src/backend/riscv/instruction_select.rs` |
| RISC-V optimizer | 5 | `src/backend/riscv/optimizer.rs` |
| RISC-V peephole | 9 | `src/backend/riscv/peephole.rs` |

### Tests de integración — 220 tests

Se ejecutan como `[[test]]` independientes definidos en `Cargo.toml`:

| Nombre | Path | Tests | Propósito |
|---|---|---|---|
| `frontend_lexer` | `test/test-frontend/lexer_tests.rs` | 18 | Tokenización |
| `frontend_parser` | `test/test-frontend/parser_tests.rs` | 29 | Parsing correcto |
| `frontend_parser_errors` | `test/test-frontend/parser_errors_tests.rs` | 9 | Errores de parsing |
| `frontend_scope` | `test/test-frontend/scope_tests.rs` | 32 | Ámbito de variables |
| `frontend_schema` | `test/test-frontend/schema_tests.rs` | 4 | Schema JSON del AST |
| `frontend_cli` | `test/test-frontend/cli_tests.rs` | 5 | CLI (usage, errores, dump-ast) |
| `middle_end_types` | `test/test-middle-end/types_tests.rs` | 11 | Sistema de tipos |
| `middle_end_unification` | `test/test-middle-end/unification_tests.rs` | 16 | Unificación Robinson |
| `middle_end_inference` | `test/test-middle-end/inference_tests.rs` | 31 | Inferencia HM |
| `middle_end_monomorphizer` | `test/test-middle-end/monomorphizer_tests.rs` | 8 | Monomorfización |
| `middle_end_lambda_lifter` | `test/test-middle-end/lambda_lifter_tests.rs` | 9 | Lambda lifting |
| `middle_end_ir_generator` | `test/test-middle-end/ir_generator_tests.rs` | 20 | Generación de IR |
| `middle_end_pipeline` | `test/test-middle-end/pipeline_tests.rs` | 8 | Pipeline completo (middle) |
| `middle_end_cli` | `test/test-middle-end/cli_tests.rs` | 2 | CLI modo check |
| `backend_codegen` | `test/test-backend/backend_tests.rs` | 13 | Codegen x86_64 (7 pasan, 6 ignorados en ARM) |
| `backend_pipeline_cli` | `test/test-backend/pipeline_cli_tests.rs` | 6 | Pipeline completo CLI (4 pasan, 2 ignorados) |
| `backend_cross_codegen` | `test/test-backend/cross_backend_tests.rs` | 10 | Codegen arm64 + riscv |

## Tests ignorados en ARM64

8 tests se saltan en ARM64 (Apple Silicon) porque generan assembly x86_64 que no se puede ensamblar en ese host:

- 6 de `backend_codegen` (usan `assemble_link_and_run` con gcc)
- 2 de `backend_pipeline_cli` (usan `assemble_and_run` con gcc)

Los tests de `backend_cross_codegen` para arm64 y riscv **no** se saltan — verifican el texto assembly sin invocar gcc.

## Cobertura

| Componente | Tests |
|---|---|
| Reader / SExpr | 18 |
| Macro expansion | 14 |
| Type inference (TypedAST) | 5 |
| Borrow checker | 5 |
| CFG + liveness | (integrado en borrow_checker) |
| Arm64 instruction select | (integrado en cross_codegen) |
| Arm64 materializer | 4 |
| Arm64 register allocator | 4 |
| Arm64 scheduler | 4 |
| Arm64 codegen (integrado) | 4 |
| RiscV instruction select | 5 |
| RiscV optimizer (strength + LVN) | 5 |
| RiscV peephole / RVC | 10 |

## Cómo ejecutar

```bash
# Todo
cargo test --release

# Solo un suite
cargo test --release --test backend_cross_codegen

# Solo un módulo (unit tests)
cargo test --release --lib frontend::sexpr::tests

# Sin release (más rápido)
cargo test
```
