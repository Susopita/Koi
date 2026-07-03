#!/usr/bin/env bash
#
# run_benchmarks.sh
#
# Corre el mismo conjunto de programas equivalentes en koi, Rust, Go y Carp,
# midiendo tiempo de compilación, tiempo de ejecución y tamaño de la
# sección .text del binario resultante. Produce results/benchmarks.csv,
# que luego se visualiza con plot_results.py.
#
# Requiere: hyperfine, size (binutils), y los 4 toolchains instalados.
# Uso: ./run_benchmarks.sh

set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$ROOT_DIR/results"
BIN_DIR="$ROOT_DIR/results/bin"
mkdir -p "$RESULTS_DIR" "$BIN_DIR"

CSV_FILE="$RESULTS_DIR/benchmarks.csv"
HYPERFINE_RUNS=15
HYPERFINE_WARMUP=3

# ---------------------------------------------------------------------------
# AJUSTAR: comandos de compilación para cada lenguaje.
# El de koi y Carp son los más propensos a necesitar cambios según su CLI
# real. Los de Rust/Go están probados.
# ---------------------------------------------------------------------------
compile_cmd() {
    local lang="$1" src="$2" out="$3"
    case "$lang" in
        koi)
            # Pipeline real: koi-ast -> koi-ir -> koi-assembly -> gcc,
            # envuelto en koi_build.sh (ver ese archivo).
            echo "\"$ROOT_DIR/koi_build.sh\" \"$src\" \"$out\""
            ;;
        rust)
            echo "rustc -O \"$src\" -o \"$out\""
            ;;
        go)
            echo "go build -o \"$out\" \"$src\""
            ;;
        carp)
            # Carp normalmente compila via gcc internamente; --output
            # controla dónde queda el binario final (verificar con
            # `carp --help`).
            echo "carp -x \"$src\" --output \"$out\""
            ;;
    esac
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
        if ! hyperfine --warmup 1 --runs "$HYPERFINE_RUNS" \
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
        hyperfine --warmup "$HYPERFINE_WARMUP" --runs "$HYPERFINE_RUNS" \
            $ignore_failure_flag \
            --export-json "$exec_json" \
            "$out" 2> "$RESULTS_DIR/${bench}_${lang}_exec.log"

        # Tamaño de la sección .text (en bytes)
        text_size=$(size "$out" 2>/dev/null | awk 'NR==2 {print $1}')
        [[ -z "$text_size" ]] && text_size="NA"

        compile_mean=$(python3 -c "import json;d=json.load(open('$compile_json'));print(d['results'][0]['mean'])" 2>/dev/null || echo NA)
        compile_stddev=$(python3 -c "import json;d=json.load(open('$compile_json'));print(d['results'][0]['stddev'])" 2>/dev/null || echo NA)
        exec_mean=$(python3 -c "import json;d=json.load(open('$exec_json'));print(d['results'][0]['mean'])" 2>/dev/null || echo NA)
        exec_stddev=$(python3 -c "import json;d=json.load(open('$exec_json'));print(d['results'][0]['stddev'])" 2>/dev/null || echo NA)

        echo "$bench,$lang,$compile_mean,$compile_stddev,$exec_mean,$exec_stddev,$text_size" >> "$CSV_FILE"
    done
done

echo ""
echo "Listo. Resultados en: $CSV_FILE"
echo "Genera las gráficas con: python3 plot_results.py"
