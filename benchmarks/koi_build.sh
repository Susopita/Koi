#!/usr/bin/env bash
# Wrapper del pipeline koi unificado para run_benchmarks.sh.
# Uso: koi_build.sh <archivo.koi> <binario_salida>
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$1"
OUT="$2"
KOI_BIN="$ROOT_DIR/target/release/koi"

cd "$ROOT_DIR" || exit 1

# Detectar la arquitectura nativa para elegir target por defecto
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64) TARGET="arm64" ;;
    *) TARGET="x86_64" ;;
esac
# Override con variable de entorno
TARGET="${KOI_FORCE_TARGET:-$TARGET}"

# Redirigir stderr según KOI_DEBUG
if [ -n "${KOI_DEBUG:-}" ]; then
    STDERR_REDIR="/dev/stderr"
else
    STDERR_REDIR="/dev/null"
fi

STEM=$(basename "$SRC" .koi)

# Compilar con koi
# koi build genera el binario en el CWD con el nombre del source (sin extensión)
echo "[koi_build] compilando $SRC → --target $TARGET ..."
"$KOI_BIN" build --target "$TARGET" "$SRC" 2>"$STDERR_REDIR" || true

# Mover el binario generado por koi al destino solicitado
if [ -f "$ROOT_DIR/$STEM" ] && [ -x "$ROOT_DIR/$STEM" ]; then
    mv "$ROOT_DIR/$STEM" "$OUT"
    echo "[koi_build] binario generado por koi: $OUT"
    exit 0
fi

# Fallback: ensamblar output.s manualmente con gcc
if [ -f output.s ]; then
    echo "[koi_build] ensamblando output.s con gcc..."
    if gcc -c output.s -o output.o 2>"$STDERR_REDIR" && gcc output.o -o "$OUT" 2>"$STDERR_REDIR"; then
        echo "[koi_build] ensamblado con gcc: $OUT"
        exit 0
    fi
fi

# En ARM64 no podemos ensamblar x86_64 — no es error fatal.
echo "[koi_build] WARNING: no se pudo generar binario para $SRC" >&2
exit 1
