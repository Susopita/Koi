# Benchmark harness: koi vs. Rust vs. Go vs. Carp

Compara tu compilador `koi` contra Carp (el lenguaje que inspiró tu diseño),
Go y Rust (rustc), en tres dimensiones: tiempo de compilación, tiempo de
ejecución del binario, y tamaño de la sección `.text`.

## Estructura

```
benchmarks/
├── koi/            programas .koi (sintaxis real de koi: defn/if/let/while/set!/aset!/do)
├── rust/           programas .rs  (probados, sintaxis estándar)
├── go/             programas .go  (probados, sintaxis estándar)
├── carp/           programas .carp (sintaxis a validar, no probada localmente)
├── koi_build.sh    wrapper del pipeline koi-ast -> koi-ir -> koi-assembly -> gcc
├── run_benchmarks.sh   harness bash + hyperfine
├── plot_results.py     genera las 3 gráficas con matplotlib
└── results/            se crea al correr los scripts (CSV + PNGs)
```

Los 4 benchmarks son equivalentes entre lenguajes, a la escala original:

| Archivo          | Qué mide                                                        |
|-------------------|-------------------------------------------------------------------|
| `fib`             | Overhead de llamada a función / recursión                        |
| `quicksort`       | Arreglos + mutación in-place (sorting real, n=100 000, arreglo de tamaño dinámico vía `new`) |
| `matrix_sum`      | Loops anidados + arreglo real materializado (matriz 500×500 = 250 000 celdas, escrita con `aset!` y sumada con `index`, no una fórmula cerrada) |
| `lambda_map`      | Pipeline map→filter→reduce con dos clausuras independientes que capturan variables libres (`factor`, `threshold`) — mide el costo real de la closure-conversion (fat pointers + struct de entorno) frente a los closures nativos de Rust/Go, n=1 000 000 |

`lambda_map` estuvo eliminado del set de benchmarks durante gran parte del
desarrollo porque las clausuras con captura estaban completamente rotas — se
restauró una vez corregido (ver `CRITICAL_FIXES_REPORT.md`). Limitación
encontrada al reconstruirlo: una clausura solo funciona si se crea e invoca
dentro de la misma función — pasarla como parámetro a una función genérica
(`map`/`filter`/`reduce` reutilizables) produce un segfault, así que el
pipeline en `koi/lambda_map.koi` está inlineado en `main` en vez de
factorizado (ver `LANGUAGE_EXTENSION_ROADMAP.md` para el detalle técnico).

## Pasos

### 1. Instalar dependencias

```bash
# hyperfine (medición estadística de tiempos)
sudo apt install hyperfine

# Rust, Go: probablemente ya los tienes
rustc --version
go version

# Carp: dale máximo medio día. Si no compila limpio en tu máquina,
# documenta el intento en el reporte y compara solo koi/Rust/Go —
# es información válida y honesta para el análisis.
git clone https://github.com/carp-lang/Carp && cd Carp && stack install

# Python deps para las gráficas
pip install matplotlib numpy --break-system-packages
```

### 2. Ya configurado

`run_benchmarks.sh`'s `compile_cmd()` ya apunta a `koi_build.sh` (wrapper del
pipeline real de 3 binarios + `gcc`), y `koi/*.koi` ya usa la sintaxis real de
koi (`defn`/`if`/`let`/`while`/`set!`/`aset!`/`do`/`index`) — no hace falta
ajustar nada antes de correr.

### 3. Correr el benchmark

```bash
chmod +x run_benchmarks.sh
./run_benchmarks.sh
```

Esto compila y ejecuta cada programa con `hyperfine` (15 corridas + 1-3 de
warmup), mide el tamaño de `.text` con `size`, y escribe todo a
`results/benchmarks.csv`. Si algún lenguaje falla en compilar (típicamente
Carp), esa fila queda como `NA` y el script sigue con el resto — no se
detiene todo por un fallo aislado.

### 4. Generar las gráficas

```bash
python3 plot_results.py
```

Produce `results/plots/compile_time.png`, `exec_time.png` y `text_size.png`
— barras agrupadas por benchmark y lenguaje, listas para pegar en el
reporte técnico (criterio "Comparación Comercial" de la rúbrica).

## Notas sobre rigor experimental

- **Corre los benchmarks con la máquina en reposo** (sin otras cargas
  pesadas), idealmente varias veces en distintos momentos para confirmar
  que los resultados son estables.
- `hyperfine` ya reporta media y desviación estándar — inclúyanlas en el
  reporte, no solo el promedio. Una diferencia dentro del margen de error
  no es una conclusión válida.
- Compilen Rust y Go con optimizaciones activadas (`-O` en rustc,
  `go build` ya optimiza por defecto) para que la comparación sea justa
  frente a lo que ustedes esperan que haga su propio backend de koi.
- `koi` no tiene asignación de registros real (todo valor vive en la pila,
  ver `koi-assembly/src/register_allocator.rs`), así que perder frente a
  Rust/Go en tiempo de ejecución es esperado — documentarlo con números es
  parte del "análisis técnico" que pide la rúbrica, no un fracaso.
- Si `koi` pierde por mucho en `lambda_map` específicamente, eso es
  información valiosa y esperada: los fat pointers + struct de entorno de su
  closure-conversion tienen overhead real frente a los closures nativos
  optimizados de Rust/Go. Documentarlo con números es parte del análisis
  técnico, no un fracaso.
