# Koi — Compilador Carp Multiarquitectura

Koi es un compilador para el lenguaje **Carp** (un dialecto Lisp con sintaxis S-expression, tipeado estáticamente) que genera código assembly para **x86_64**, **ARM64 (AArch64)** y **RISC-V (RV64)**. Escrito completamente en **Rust**.

```carp
(defn fib [n]
  (if (< n 2)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))

(defn main []
  (print (fib 32)))
```

## Características

- **3 backends:** x86_64, ARM64, RISC-V — seleccionables via `--target`
- **3 fases clásicas:** Frontend (Lisp reader + macros + type inference), Middle-end (SSA IR + optimizaciones), Backend (codegen + register allocation + scheduling)
- **Lisp real:** Reader de S-expressions, sistema de macros `defmacro` con compile-time DSL (`quote`, `list`, `cons`, `if`, `car`, `cdr`, `concat`, `nil?`)
- **Hindley-Milner:** Inferencia de tipos completa con unificación Robinson, monomorfización, lambda lifting
- **Borrow checker:** Análisis de ownership con detección de use-after-move y puntos de `free` automáticos
- **Optimizaciones:** Constant folding, strength reduction, DCE, if-conversion (CSEL/CSINC), list scheduling, peephole, RVC compression
- **Modo IDE:** Salida JSON estructurada para `--check` y `--dump-ast`
- **322 tests**, 0 failures

## Benchmarks vs Rust y Go

| Benchmark | Koi | Rust | Go | Gap |
|---|---|---|---|---|
| fib(32) | 32.5 ms | 9.3 ms | 14.4 ms | ~3.5x |
| quicksort (100K) | 23.1 ms | 7.7 ms | 9.1 ms | ~3.0x |
| matrix_sum (500×500) | 3.6 ms | 2.5 ms | 3.5 ms | ~1.4x |
| lambda_map (1M) | 19.5 ms | 6.6 ms | 8.0 ms | ~3.0x |

Koi compila ~3-4x más rápido que Rust y produce binarios **100-500x más pequeños** (~1.5-2.5 KB de `.text` vs cientos de KB).

## Uso rápido

```bash
# Compilar programa
koi build programa.carp

# Type-check (salida JSON)
koi build --check programa.carp

# Elegir target
koi build --target riscv programa.carp
koi build --target arm64  programa.carp

# Dump AST (salida JSON)
koi build --dump-ast programa.carp
```

## Documentación

| Documento | Contenido |
|---|---|
| [ARQUITECTURA.md](docs/ARQUITECTURA.md) | Pipeline completo, fases del compilador, decisones técnicas |
| [BUILD.md](docs/BUILD.md) | Compilar desde cero, requisitos, solución de problemas |
| [BENCHMARKS.md](docs/BENCHMARKS.md) | Resultados detallados, metodología, análisis |
| [TESTS.md](docs/TESTS.md) | Cobertura, suites, cómo ejecutar |

## Licencia

Apache 2.0
