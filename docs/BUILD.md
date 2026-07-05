# Compilar Koi

## Requisitos

- **Rust** 1.70+ (toolchain estable)
- **gcc** o **clang** (para ensamblar/enlazar el output x86_64)
- **Nix** (opcional, para el entorno reproducible)

## Compilar

```bash
# Clonar
git clone <repo> koi
cd koi

# Compilar (release)
cargo build --release

# Los binarios quedan en:
#   ./target/release/koi
```

### Con Nix (recomendado)

```bash
nix-shell
cargo build --release
```

El `shell.nix` provee `cargo`, `rustc`, `gcc`, `rustfmt`, `clippy` y `rust-analyzer`.

## Cross-compilación

Koi genera assembly textual — no necesita toolchain cruzado para compilar el *compilador*. Pero para ensamblar el output de un target no-nativo:

```bash
# Generar assembly para riscv
./target/release/koi build --target riscv programa.carp
# output.s contiene código RISC-V

# Ensamblar con toolchain cruzado (ej. riscv64-linux-gnu-gcc)
riscv64-linux-gnu-gcc -c output.s -o output.o
riscv64-linux-gnu-gcc output.o -o programa
```

## Solución de problemas

| Problema | Causa | Solución |
|---|---|---|
| `cargo: not found` | Rust no instalado | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `gcc -c failed` | ARM64 ensamblando x86_64 | Usar `--target arm64` o saltar el paso de gcc (el `output.s` se genera igual) |
| `target/release/koi: No such file` | No compilado | Ejecutar `cargo build --release` |
| Tests lentos | `--release` compila optimizado | Usar `cargo test` (debug) para iteración rápida |

## Tests

```bash
# Todos los tests
cargo test --release

# Tests de un backend específico
cargo test --release -p koi --test backend_cross_codegen

# Tests de un módulo específico
cargo test --release --lib frontend::sexpr::tests

# Sin release (más rápido para desarrollo)
cargo test
```

## Benchmark

```bash
# Desde el directorio raíz
cd benchmarks
./run_benchmarks.sh
```

Requiere `hyperfine` (instalable via `cargo install hyperfine`) y los compiladores Rust/Go.
