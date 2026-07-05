# Benchmarks — Koi vs Rust vs Go

## Entorno

| Especificación | Valor |
|---|---|
| CPU | Apple M3 (ARM64) |
| RAM | 18 GB |
| SO | macOS Sequoia 15.x |
| Rust | 1.95.0 |
| Go | 1.22.2 |
| Hyperfine | 1.20.0 |
| Koi | release build, target x86_64 |

> **Nota:** Los benchmarks se ejecutan en ARM64 (Apple Silicon). Koi genera assembly x86_64 que se ensambla y ejecuta via `gcc` (que incluye Rosetta 2 en macOS). Esto añade overhead de traducción que no está presente en Linux nativo.

## Programas

| Benchmark | Descripción | Escala |
|---|---|---|
| **fib** | Fibonacci recursivo `fib(32)` | Mide overhead de llamadas a función |
| **quicksort** | Quicksort in-place con LCG, arreglo dinámico | n=100,000 |
| **matrix_sum** | Matriz 500×500 materializada con `aset!`/`index` | 250,000 celdas |
| **lambda_map** | Pipeline map→filter→reduce con 2 closures | n=1,000,000 |

## Metodología

- 15 corridas medidas + 1-3 de warmup por benchmark
- Máquina en reposo (sin carga background)
- `hyperfine` con `--ignore-failure` para Koi (usa el valor de retorno como exit code)
- Verificación de corrección contra fórmulas cerradas y versiones Rust/Go

## Resultados

### Tiempo de compilación (media ± σ)

| Benchmark | Koi | Rust | Go | Carp |
|---|---|---|---|---|
| fib | **31.6 ± 1.4 ms** | 124.9 ± 30.1 ms | 76.1 ± 7.6 ms | NA |
| quicksort | **34.4 ± 2.0 ms** | 123.2 ± 5.0 ms | 45.3 ± 1.9 ms | NA |
| matrix_sum | **30.9 ± 1.4 ms** | 136.8 ± 3.4 ms | 47.6 ± 5.3 ms | NA |
| lambda_map | **34.9 ± 3.6 ms** | 124.8 ± 2.2 ms | 46.6 ± 2.9 ms | NA |

Koi compila **3–4x más rápido** que Rust y **~1.5x más rápido** que Go.

### Tiempo de ejecución (media ± σ)

| Benchmark | Koi | Rust | Go | Gap vs Rust |
|---|---|---|---|---|
| fib (32) | 32.5 ± 2.4 ms | 9.3 ± 0.4 ms | 14.4 ± 1.9 ms | **~3.5x** |
| quicksort (100K) | 23.1 ± 0.9 ms | 7.7 ± 0.7 ms | 9.1 ± 0.6 ms | **~3.0x** |
| matrix_sum (500×500) | 3.6 ± 0.2 ms | 2.5 ± 0.3 ms | 3.5 ± 0.4 ms | **~1.4x** |
| lambda_map (1M) | 19.5 ± 1.2 ms | 6.6 ± 0.3 ms | 8.0 ± 0.2 ms | **~3.0x** |

### Tamaño de `.text` (bytes)

| Benchmark | Koi | Rust | Go |
|---|---|---|---|
| fib | **1,454** | 327,579 | 1,186,150 |
| quicksort | **2,562** | 328,827 | 1,187,087 |
| matrix_sum | **1,974** | 329,187 | 1,186,992 |
| lambda_map | **2,280** | 328,451 | 1,186,039 |

Koi produce binarios **100–500x más pequeños** — no enlaza runtime estático.

## Análisis

### Causa raíz del gap de ejecución

El gap de 2–3.5x frente a Rust tiene una causa principal: **todo valor vive en la pila** en el register allocator original (x86_64). Cada operación aritmética hace recarga desde memoria en vez de mantener valores en registros. El nuevo register allocator (implementado en ARM64/RISC-V) mitiga parcialmente esto, pero el backend x86_64 aún usa el linear scan básico sin spilling selectivo.

### matrix_sum casi empata

Cuando el workload es dominado por acceso secuencial a memoria (matrix_sum), la desventaja de Koi se reduce drásticamente (~1.4x) porque Rust/Go *también* terminan tocando memoria para cada acceso al arreglo.

### Ventajas de Koi

- **Compilación más rápida** que Rust y Go en todos los benchmarks
- **Binarios órdenes de magnitud más pequeños** (~1.5–2.5 KB de `.text`)
- Sin dependencias externas en runtime

### Carp (ausente)

No se pudo instalar el compilador Carp real (requiere toolchain Haskell completo desde fuente). Todas las filas son NA.

## Ejecutar los benchmarks

```bash
cd benchmarks
./run_benchmarks.sh

# Resultados en:
#   results/benchmarks.csv
#   results/plots/compile_time.png
#   results/plots/exec_time.png
#   results/plots/text_size.png
```
