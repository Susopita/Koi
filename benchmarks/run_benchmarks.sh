#!/usr/bin/env bash
#
# run_benchmarks.sh
#
# Corre el mismo conjunto de programas equivalentes en koi, Rust, Go y Carp,
# midiendo tiempo de compilación, tiempo de ejecución y tamaño de la
# sección .text del binario resultante. Produce results/benchmarks.csv,
# que luego se visualiza con plot_results.py.
#
# Uso: ./run_benchmarks.sh
#
# IMPORTANTE: Corre este script FUERA de nix-shell. El script usa el
# compilador koi + gcc del sistema (que linkea correctamente en macOS).
# Para Rust/Go/hyperfine invoca `nix-shell` automaticamente cuando
# esos toolchains no estan disponibles en el PATH del sistema.

set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$ROOT_DIR/results"
BIN_DIR="$ROOT_DIR/results/bin"
mkdir -p "$RESULTS_DIR" "$BIN_DIR"

CSV_FILE="$RESULTS_DIR/benchmarks.csv"
HYPERFINE_RUNS=15
HYPERFINE_WARMUP=3

# Ruta al shell.nix para invocar toolchains via nix-shell
NIX_SHELL_DIR="$ROOT_DIR/.."

# ---------------------------------------------------------------------------
# Helpers: wrappers que usan nix-shell solo si el toolchain no esta en el
# PATH del sistema. Esto permite correr el script FUERA de nix-shell:
# koi + gcc del sistema linkean contra libSystem, rustc/go/hyperfine
# se obtienen via nix-shell cuando no estan instalados globalmente.
# ---------------------------------------------------------------------------
# Wrapper para invocar algo dentro de nix-shell (solo si no esta en PATH)
nix_shell_wrap() {
    local cmd="$1"
    if command -v "$cmd" &>/dev/null; then
        # Ya está en PATH del sistema, llamar directamente
        echo "$*"
    else
        # No está en PATH, invocar via nix-shell
        echo "nix-shell \"$NIX_SHELL_DIR/shell.nix\" --pure --run \"$*\""
    fi
}

compile_cmd() {
    local lang="$1" src="$2" out="$3"
    case "$lang" in
        koi)
            # Pipeline real: koi -> as -> gcc del sistema (linkea contra
            # libSystem del SISTEMA, no contra el clang de Nix).
            # koi_build.sh corre con gcc del PATH del sistema.
            echo "\"$ROOT_DIR/koi_build.sh\" \"$src\" \"$out\""
            ;;
        rust)
            # rustc puede no estar en PATH -> nix-shell
            echo "$(nix_shell_wrap rustc) -O \"$src\" -o \"$out\""
            ;;
        go)
            # go puede no estar en PATH -> nix-shell
            echo "$(nix_shell_wrap go) build -o \"$out\" \"$src\""
            ;;
        carp)
            echo "carp -x \"$src\" --output \"$out\""
            ;;
    esac
}

# Wrapper para hyperfine: usa nix-shell si no esta en PATH del sistema
run_hyperfine() {
    if command -v hyperfine &>/dev/null; then
        hyperfine "$@"
    else
        nix-shell "$NIX_SHELL_DIR/shell.nix" --pure --run "hyperfine $*"
    fi
}

# 'size' no esta en nix-shell, usamos stat -f%z en macOS o size del sistema
get_text_size() {
    local bin="$1"
    if command -v size &>/dev/null; then
        size "$bin" 2>/dev/null | awk 'NR==2 {print $1}'
    elif [[ "$(uname)" == "Darwin" ]]; then
        # En macOS usamos el tamaño total del binario como aproximacion
        # (no hay seccion .text directamente con stat)
        stat -f%z "$bin" 2>/dev/null || echo "NA"
    else
        echo "NA"
    fi
}

BENCHMARKS=(fib quicksort matrix_sum lambda_map)
declare -A EXT=( [koi]="koi" [rust]="rs" [go]="go" [carp]="carp" )
LANGS=(koi rust go carp)

echo "benchmark,language,compile_mean_s,compile_stddev_s,exec_mean_s,exec_stddev_s,text_size_bytes" > "$CSV_FILE"

for bench in "${BENCHMARKS[@]}"; do
    for lang in "${LANGS[@]}"; do
        src="$ROOT_DIR/$lang/${bench}.${EXT[$lang]}"
        out="$BIN_DIR/${bench}_${lang}"

        if [[ ! -f "$src" ]]; then
            echo "  [skip] $src no existe"
            continue
        fi

        cmd=$(compile_cmd "$lang" "$src" "$out")
        echo "== Compilando $bench ($lang) =="

        compile_json="$RESULTS_DIR/${bench}_${lang}_compile.json"
        if ! run_hyperfine --warmup 1 --runs "$HYPERFINE_RUNS" \
                --export-json "$compile_json" \
                "$cmd" 2> "$RESULTS_DIR/${bench}_${lang}_compile.log"; then
            echo "  [FALLO] compilación de $bench en $lang -- ver ${bench}_${lang}_compile.log"
            echo "$bench,$lang,NA,NA,NA,NA,NA" >> "$CSV_FILE"
            continue
        fi

        if [[ ! -x "$out" ]]; then
            echo "  [FALLO] no se generó binario ejecutable para $bench/$lang"
            echo "$bench,$lang,NA,NA,NA,NA,NA" >> "$CSV_FILE"
            continue
        fi

        echo "== Ejecutando $bench ($lang) =="
        exec_json="$RESULTS_DIR/${bench}_${lang}_exec.json"
        # koi no tiene una convencion de "return 0"; el valor que retorna `main`
        # se usa directamente como codigo de salida del proceso (ver koi-assembly
        # codegen), asi que un exit code distinto de 0 es normal y no indica un
        # fallo real de ejecucion. --ignore-failure evita que hyperfine aborte
        # la medicion por eso.
        ignore_failure_flag=""
        [[ "$lang" == "koi" ]] && ignore_failure_flag="--ignore-failure"
        run_hyperfine --warmup "$HYPERFINE_WARMUP" --runs "$HYPERFINE_RUNS" \
            $ignore_failure_flag \
            --export-json "$exec_json" \
            "$out" 2> "$RESULTS_DIR/${bench}_${lang}_exec.log"

        # Tamaño del binario (aprox)
        text_size=$(get_text_size "$out")
        [[ -z "$text_size" ]] && text_size="NA"

        # python3 puede estar en PATH del sistema o en nix-shell
        PYTHON_CMD="python3"
        if ! command -v python3 &>/dev/null; then
            PYTHON_CMD="nix-shell \"$NIX_SHELL_DIR/shell.nix\" --pure --run python3"
        fi

        compile_mean=$($PYTHON_CMD -c "import json;d=json.load(open('$compile_json'));print(d['results'][0]['mean'])" 2>/dev/null || echo NA)
        compile_stddev=$($PYTHON_CMD -c "import json;d=json.load(open('$compile_json'));print(d['results'][0]['stddev'])" 2>/dev/null || echo NA)
        exec_mean=$($PYTHON_CMD -c "import json;d=json.load(open('$exec_json'));print(d['results'][0]['mean'])" 2>/dev/null || echo NA)
        exec_stddev=$($PYTHON_CMD -c "import json;d=json.load(open('$exec_json'));print(d['results'][0]['stddev'])" 2>/dev/null || echo NA)

        echo "$bench,$lang,$compile_mean,$compile_stddev,$exec_mean,$exec_stddev,$text_size" >> "$CSV_FILE"
    done
done

echo ""
echo "Listo. Resultados en: $CSV_FILE"
echo "Genera las gráficas con: python3 plot_results.py"
