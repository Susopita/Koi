# Koi Compiler — Language Feature Reference

> Compilador Carp → ARM64 / x86_64 / RISC-V.
> Documento autocontenido para entender qué features del lenguaje Carp
> soporta koi, cuáles están en desarrollo y cuáles no existen.

---

## Convenciones

| Símbolo | Significado |
|---|---|
| ✅ | Implementado y probado |
| 🔄 | En desarrollo (parcial o con bugs conocidos) |
| ❌ | No implementado |
| 🔲 | Planeado pero no iniciado |

---

## 1. Literales y Tipos Primitivos

| Feature | Sintaxis | Tipo | Estado |
|---|---|---|---|
| Entero 64-bit | `42`, `0`, `-7` | `i64` | ✅ |
| Flotante 64-bit | `3.14`, `-2.5` | `f64` | ✅ |
| Booleano | `true`, `false` | `bool` | ✅ |
| String | `"hello"` | `string` | ✅ |
| Carácter | `'a'` | `char` | ❌ |
| Byte | `1b` | `byte` | ❌ |
| Long | `1500l` | `long` (alias i64) | ❌ |
| Float 32-bit | `3.14f` | `float` | ❌ |
| Suffix `b` / `l` / `f` | `42l` | — | ❌ |

---

## 2. Type System

| Feature | Sintaxis | Estado |
|---|---|---|
| `i64` | Tipo entero | ✅ |
| `f64` | Tipo flotante | ✅ |
| `bool` | Booleano | ✅ |
| `string` | String | ✅ |
| `Array(T)` vía `arr_T` | `arr_i64`, `arr_f64` | ✅ |
| `(Array T)` Carp-style | `(Array Int64)` | 🔲 |
| `Pointer(T)` vía `ptr_T` | `ptr_i64` | ✅ |
| `(Ptr T)` Carp-style | `(Ptr Int64)` | 🔲 |
| `Struct(name)` | Structs definidos por usuario | ✅ |
| `Function { params, ret }` | Tipos función | ✅ |
| `TypeVar` | Variables de inferencia | ✅ |
| `Unit` | Tipo unit | ✅ |
| Anotación `:type` en params | `[x :i64]` | ✅ |
| `(Fn [args] ret)` syntax | `(Fn [Int64] Int64)` | ✅ |
| `(-> param ret)` syntax | `(-> Int64 Int64)` | ✅ |

---

## 3. Structs (User-defined Types)

| Feature | Sintaxis | Estado |
|---|---|---|
| Struct definition (koi-style) | `(defstruct Point [x i64] [y i64])` | ✅ |
| Struct definition (Carp-style) | `(deftype Point [x i64, y i64])` | ✅ |
| Heap allocation | `(new Point)` | ✅ |
| Field read | `(field obj field)` | ✅ |
| Field write | `(set-field! obj field val)` | ✅ |

---

## 4. Funciones

| Feature | Sintaxis | Estado |
|---|---|---|
| Definición | `(defn name [params] body)` | ✅ |
| Parámetros tipados | `[x :i64 y :f64]` | ✅ |
| Forward references | Llamar función definida después | ✅ |
| Recursión mutua | `f` llama `g`, `g` llama `f` | ✅ |
| Auto-recursión | `(defn f [x] (f (- x 1)))` | ✅ |
| Tail Call Optimization | Auto-recursión en tail position | 🔲 |
| Lambda `lambda` | `(lambda [x] (+ x 1))` | ✅ |
| Lambda `fn` (Carp-style) | `(fn [x] (+ x 1))` | ✅ |
| Lambda con tipos | `(lambda [x :i64] (+ x 1))` | ✅ |
| Closures (captura libre) | `(let [x 1] (lambda [y] (+ y x)))` | ✅ |
| Closure call | `(mi-closure arg)` | ✅ |
| Closure como argumento | `(apply (lambda [x] (* x 2)) 5)` | ✅ |
| Closure como retorno | `(defn f [] (lambda [y] (+ y 1)))` | ✅ |

---

## 5. Variables y Bindings

| Feature | Sintaxis | Estado |
|---|---|---|
| Let binding (Carp-style, simultáneo) | `(let [x 1 y 2] (+ x y))` | ✅ |
| Let binding secuencial (let*) | No soportado (usar nested let) | ✅* |
| `set!` (mutación) | `(set! x (+ x 1))` | ✅ |
| Variable global `def` | `(def pi 3.14)` | 🔲 |

> *Nota: koi solía tener let secuencial (let*). Se cambió a simultáneo
> (Carp-style). Bindings en el mismo `let` no se ven entre sí. Usar
> `(let [a 1] (let [b (+ a 1)] ...))` para dependencias.

---

## 6. Control Flow

| Feature | Sintaxis | Estado |
|---|---|---|
| If | `(if cond then else)` | ✅ |
| If sin else | `(if cond then)` | ✅ |
| While | `(while cond body)` | ✅ |
| Loop | `(loop [i 0] (< i n) (+ i 1) body)` | ✅ |
| Do | `(do expr1 expr2 ...)` | ✅ |
| `cond` multi-branch | `(cond (c1) e1 (c2) e2 else)` | 🔲 |

---

## 7. Pointer Operations

| Feature | Sintaxis | Tipo | Estado |
|---|---|---|---|
| Address-of | `(& x)` o `(ref x)` | `Pointer(T)` | ✅ |
| Dereference | `(* x)` | `T` | ✅ |
| Copy | `(@ x)` o `(copy x)` | `T` | 🔄 |
| Heap allocation | `(new type [size])` | `Pointer(T)` | ✅ |

---

## 8. Array Operations

| Feature | Sintaxis | Estado |
|---|---|---|
| Array literal | `[1 2 3]` | ✅ |
| Index read | `(index arr i)` | ✅ |
| Index write | `(aset! arr i val)` | ✅ |

---

## 9. Builtin Operators

| Op | Soporte | Op | Soporte |
|---|---|---|---|
| `+` | ✅ | `<` | ✅ |
| `-` | ✅ | `<=` | ✅ |
| `*` | ✅ | `>` | ✅ |
| `/` | ✅ | `>=` | ✅ |
| `==` | ✅ | `!=` | ✅ |
| `&&` | ✅ | `\|\|` | ✅ |
| `!` (not) | ✅ | `print` | ✅ |
| `malloc` | ✅ | `free` | ✅ |

---

## 10. Ownership / Borrow Checking

| Feature | Estado |
|---|---|
| Ownership transfer (non-Copy types moved on call) | ✅ |
| Copy types (i64, f64, bool, string — never moved) | ✅ |
| Immutable borrow via `&` / `ref` | ✅ |
| Drop injection at end of scope | ✅ |
| Use-after-move detection | ✅ |
| Double borrow detection | ✅ |
| Closure capture moves | ✅ |
| If-merge ownership | ✅ |

---

## 11. Macros

| Feature | Sintaxis | Estado |
|---|---|---|
| Macro definition | `(defmacro name [params & rest] body)` | ✅ |
| Quote | `(quote x)` / `'x` | ✅ |
| Car / Cdr | `(car xs)` / `(cdr xs)` | ✅ |
| Cons | `(cons x xs)` | ✅ |
| Concat | `(concat xs ys)` | ✅ |
| Nil? | `(nil? x)` | ✅ |
| List construction | `(list a b c)` | ✅ |
| Rest params | `& rest` | ✅ |
| Fixpoint expansion | Macro dentro de macro | ✅ |

---

## 12. Compiler Pipeline

| Fase | Descripción | Estado |
|---|---|---|
| Scanner | Tokens desde texto fuente | ✅ |
| Parser | AST desde tokens | ✅ |
| Scope Analysis | Declaración de variables | ✅ |
| Type Inference | Hindley-Milner + unificación | ✅ |
| Lambda Lifting | Closure conversion | ✅ |
| Macro Expansion | Macros en tiempo de compilación | ✅ |
| Borrow Checker | Ownership analysis | ✅ |
| IR Generation | SSA IR | ✅ |
| Optimizer | Constant folding, DCE, strength reduction | ✅ |
| Backend x86_64 | AT&T assembly + linear scan regalloc | ✅ |
| Backend ARM64 | Maximal Munch + Chaitin-Briggs + list scheduling | ✅ |
| Backend RISC-V | Maximal Munch + linear scan + RVC | ✅ |
| Assembly + Link | Genera binario ejecutable | ✅ |

---

## 13. Compiler Flags y Modos

| Flag | Descripción | Estado |
|---|---|---|
| `build <file>` | Compilar a binario | ✅ |
| `--check` | Type-check only | ✅ |
| `--dump-ast` | JSON AST a stdout | ✅ |
| `--target arm64\|x86_64\|riscv` | Arquitectura destino | ✅ |
| `--no-optimize` | Skip optimizer + scheduler + peephole | ✅ |

---

## 14. Resumen por Prioridad

### Implementado (✅)
- Tipos: i64, f64, bool, string
- Structs (defstruct + deftype)
- Funciones (defn, fn, lambda, closures, recursión)
- Let (simultáneo), set!, if, while, loop, do
- Arrays (new, index, aset!)
- Punteros (&, *)
- Builtins aritméticos/comparación/lógicos/print/malloc/free
- Sistema de ownership (borrow checker)
- Macros (defmacro, quote, car/cdr/cons)
- 3 backends (ARM64, x86_64, RISC-V)
- Optimizer IR-level

### En desarrollo (🔄)
- Copy (@ / copy)

### Planeado (🔲)
- ref / copy special forms
- Array type syntax (Array T)
- the special form
- cond multi-branch
- Tail Call Optimization
- def variables globales
- Tipos Byte, Char, Long, Float
- Sufijos numéricos
- Entry point flexible
- Project system / multi-file

### No implementado (❌)
- Sum types / pattern matching
- Interfaces / traits
- Módulos (defmodule, use)
- C Interop
- REPL
