#!/usr/bin/env bash
# Wrapper del pipeline real de koi (3 binarios via /tmp + gcc) para que
# run_benchmarks.sh pueda invocarlo como un solo comando de compilación.
# Uso: koi_build.sh <archivo.koi> <binario_salida>
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$1"
OUT="$2"

# koi-assembly escribe "output.s" (y koi-ast/koi-ir leen/escriben /tmp/ast.json,
# /tmp/ir.json) relativo al CWD del proceso, asi que fijamos el CWD aqui en vez
# de asumir que el invocador (p.ej. hyperfine) ya esta parado en ROOT_DIR.
cd "$ROOT_DIR" || exit 1

"$ROOT_DIR/target/release/koi-ast" "$SRC" || exit 1
"$ROOT_DIR/target/release/koi-ir" || exit 1
"$ROOT_DIR/target/release/koi-assembly" || exit 1
gcc -c "$ROOT_DIR/output.s" -o "$ROOT_DIR/output.o" || exit 1
gcc "$ROOT_DIR/output.o" -o "$OUT" || exit 1
