# Arquitectura de Koi

## Visión General

Koi es un compilador de 3 etapas que transforma código fuente Carp en assembly para tres arquitecturas. El pipeline completo opera **en memoria** usando estructuras Rust nativas — no hay archivos temporales en `/tmp/`.

```
┌─────────────────────────────────────────────────────────────────────┐
│                    FRONTEND (koi::frontend)                         │
│                                                                     │
│  Source (.carp)                                                     │
│    → Scanner (char-by-char → tokens con línea/columna)              │
│    → Reader (tokens → SExpr — árbol puramente sintáctico)           │
│    → Macro Expander (defmacro → quote/list/cons/if/car/cdr)         │
│    → TypedAST Builder (SExpr → TypedExpr con Type::Variable)        │
│    → Type Inferer (Hindley-Milner constraints → unificación)        │
│    → Borrow Checker (ownership → use-after-move → drop points)      │
│    → ASTNode (para compatibilidad con pipeline legacy)              │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ ASTNode (en memoria)
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    MIDDLE-END (koi::middle_end)                     │
│                                                                     │
│  ASTNode                                                             │
│    → ConstraintGenerator (HM type constraints)                      │
│    → Unifier (Robinson unification)                                 │
│    → Monomorphizer (especialización de genéricos)                   │
│    → Lambda Lifter (closure conversion → MakeClosure + env struct)  │
│    → IR Generator (AST → SSA con BasicBlock, Phi, Branch)           │
│    → Optimizer (constant folding, strength reduction, DCE)          │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ IRProgram (en memoria)
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    BACKEND (koi::backend)                           │
│                                                                     │
│  TargetArch dispatch → &dyn TargetBackend                          │
│                       │                                             │
│          ┌────────────┼─────────────┐                               │
│          ▼            ▼             ▼                               │
│      x86_64        arm64         riscv                             │
│          │            │             │                               │
│          │    Instruction Select   │                               │
│          │    Materializer         │                               │
│          │    Register Allocator   │                               │
│          │    List Scheduler       │                               │
│          │    Peephole + RVC       │                               │
│          ▼            ▼             ▼                               │
│      output.s     output.s      output.s                           │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 1. Frontend

### 1.1 Scanner (`src/frontend/scanner.rs`)

Escáner character-by-character que produce `TokenWithPos { token, line, column }`.

| Token | Ejemplo |
|---|---|
| `LParen` / `RParen` | `(`, `)` |
| `LBracket` / `RBracket` | `[`, `]` |
| `IntLiteral(i64)` | `42`, `-7` |
| `FloatLiteral(f64)` | `3.14` |
| `StringLiteral(String)` | `"hola"` |
| `Symbol(String)` | `defn`, `+`, `x`, `kebab-case?` |
| `Colon`, `Arrow`, `Ampersand`, `Asterisk` | `:`, `->`, `&`, `*` |

Los comentarios `;` se ignoran completamente.

### 1.2 Reader / SExpr (`src/frontend/sexpr.rs`)

El reader convierte el flujo de tokens en un árbol puramente sintáctico:

```rust
pub enum SExpr {
    Symbol(String),
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    List(Vec<SExpr>),
}
```

No realiza ninguna validación semántica — solo agrupa por paréntesis. Los paréntesis `()`, corchetes `[]` y llaves `{}` son equivalentes.

### 1.3 Macro Expander (`src/frontend/macro_expander.rs`)

Registra formas `(defmacro nombre [params & rest] cuerpo...)` y las expande.

**DSL compile-time disponible en cuerpos de macro:**

| Forma | Efecto |
|---|---|
| `(quote x)` | Devuelve x literal |
| `(list e1 e2..)` | Evalúa y recolecta en List |
| `(cons e1 e2)` | Antepone e1 a e2 (lista) |
| `(if cond then else)` | Rama condicional |
| `(car xs)` / `(cdr xs)` | Primero / resto |
| `(concat xs ys..)` | Concatena listas |
| `(nil? x)` | True si lista vacía |

Las macros se expanden con punto fijo: una macro que expande a código que contiene otra macro se re-expande automáticamente.

### 1.4 TypedAST (`src/frontend/typed_ast.rs`)

Árbol AST con tipos, donde cada nodo tiene un campo `Type`. Se construye desde `SExpr` con tipos inicializados como `Type::Variable(TypeVar::fresh())`.

```rust
pub enum TypedExpr {
    Int(i64, Type), Float(f64, Type), Bool(bool, Type), Str(String, Type),
    Var(String, Type),
    Let(Vec<(String, TypedExpr)>, Box<TypedExpr>, Type),
    Set(String, Box<TypedExpr>, Type),
    Lambda(Vec<(String, Option<Type>)>, Box<TypedExpr>, Type),
    App(Box<TypedExpr>, Vec<TypedExpr>, Type),
    If(Box<TypedExpr>, Box<TypedExpr>, Option<Box<TypedExpr>>, Type),
    While(Box<TypedExpr>, Box<TypedExpr>, Type),
    Loop { variable, init, condition, step, body, ty },
    Do(Vec<TypedExpr>, Type),
    Array(Vec<TypedExpr>, Type),
    New { type_str, size_or_init, ty },
    Field(Box<TypedExpr>, String, Type),
    SetField(Box<TypedExpr>, String, Box<TypedExpr>, Type),
    Index(Box<TypedExpr>, Box<TypedExpr>, Type),
    AddrOf(Box<TypedExpr>, Type),
    Deref(Box<TypedExpr>, Type),
}
```

### 1.5 Type Inferer (`src/frontend/type_inferer.rs`)

Hindley-Milner con generación de constraints por nodo:

| Nodo | Constraints |
|---|---|
| `Int(_, t)` | `t = Int64` |
| `App(f, args, t)` | `type(f) = Function([type(args)], t)` |
| `If(cond, then, else, t)` | `type(cond) = Bool`, `type(then) = type(else) = t` |
| `While(cond, body, t)` | `type(cond) = Bool`, `t = Unit` |
| `Lambda(params, body, fn_ty)` | `fn_ty = Function([param_types], type(body))` |
| `AddOf(op, t)` | `t = Pointer(type(op))` |
| `Deref(op, t)` | `type(op) = Pointer(t)` |
| `Index(arr, idx, t)` | `type(arr) = Array(t)`, `type(idx) = Int64` |

Builtins con reglas precisas: `+`/`-`/`*`/`/` (arith), `<`/`>`/`==`/`!=` (cmp → Bool), `print`, `malloc`, `free`, `aset!`.

### 1.6 Borrow Checker (`src/frontend/borrow_checker.rs`)

Tres fases por función:
1. **CFG construction:** construye `BasicBlock`s con enlaces para `if`/`while`/`loop`
2. **Liveness (backward dataflow):** `live_in`/`live_out` por bloque
3. **Ownership checking + Drop injection:** detecta use-after-move y referencias superpuestas

Reglas:
- Primitivas (`+`, `-`, `*`, `print`, `malloc`, etc.) no consumen ownership
- Llamadas a función transfieren ownership de argumentos
- `(let [b a] ...)` mueve `a` a `b`
- `&x` / `(field obj f)` → borrow inmutable
- `(set-field! obj f val)` → borrow mutable
- Variables aún owned al final del scope reciben un `free` automático

---

## 2. Middle-end

### 2.1 Sistema de tipos (`src/middle_end/types.rs`)

```rust
pub enum Type {
    Int64, Float64, Bool, String,
    Array(Box<Type>), Pointer(Box<Type>),
    Struct(String),
    Function { params: Vec<Type>, return_type: Box<Type> },
    Variable(TypeVar),
    Unit,
}
```

`TypeVar` con contador atómico para IDs únicos. `Substitution` con `occurs_check`, `apply`, `bind`.

### 2.2 Unificación (`src/middle_end/unification.rs`)

Unificador de Robinson con walk sobre tipos compuestos. Variables no resueltas se resuelven a `Int64` por defecto (convención MVP de 64 bits).

### 2.3 IR (`src/middle_end/ir.rs`)

Representación intermedia en SSA con bloques:

```rust
pub struct IRProgram { pub ir_type: String, pub functions: Vec<IRFunction> }
pub struct IRFunction { pub name, pub return_type, pub parameters, pub blocks }
pub struct BasicBlock { pub label, pub instructions }
pub enum Instruction {
    Const, BinOp, Call, CallIndirect, Return, Jump, Branch,
    Phi, Alloc, GetField, SetField, GetIndex, SetIndex, AddrOf, Deref,
}
```

Cada instrucción tiene `result_name()` y `result_type()` para el backend.

### 2.4 Pipeline (`src/middle_end/pipeline.rs`)

```
inference → unification → monomorphization → lambda lifting → IR generation
```

---

## 3. Backend

### 3.1 Arquitectura multi-target

```rust
pub trait TargetBackend {
    fn name(&self) -> &'static str;
    fn generate_code(&self, program: &IRProgram) -> Result<String, CompileError>;
}
```

Dispatch via `backend_for(arch)` usando `OnceLock` por arquitectura.

### 3.2 x86_64

| Fase | Implementación |
|---|---|
| Codegen | Generador x86-64 AT&T con soporte para int/float/string, structs, closures, llamadas indirectas |
| Register Allocator | Linear Scan con 14 GP registers + spilling a stack |
| Optimizer IR | Constant folding, strength reduction (mul/div por pow2 → shifts), DCE, unreachable blocks |
| Peephole | Self-moves, no-ops, store-then-reload, dead jumps |

### 3.3 ARM64 (AArch64)

| Fase | Módulo | Algoritmo |
|---|---|---|
| 1 | `instruction_select.rs` | **Maximal Munch**: patrones de barrel shifter (add/shift), CTZ, strength reduction. **If-conversion**: Branch → simple blocks → Phi → `CSEL`/`CSINC`, eliminando branches |
| 2 | `materializer.rs` | **Greedy MOVZ/MOVK**: segmenta constantes 64-bit en 4 chunks de 16 bits, emite la secuencia mínima |
| 3 | `register_allocator.rs` | **Chaitin-Briggs graph coloring**: 28 registros disponibles (x0–x28). Simplify/Select con spilling. **Coalescencia**: asigna colores adyacentes para `LDP`/`STP` |
| 4 | `scheduler.rs` | **List scheduling**: DAG de dependencias RAW/WAR/WAW, prioridad por critical path + cycle_ready, tabla de latencias Cortex-A72 |
| 5 | `peephole.rs` | Optimizaciones post-emisión |

### 3.4 RISC-V (RV64)

| Fase | Módulo | Algoritmo |
|---|---|---|
| 1 | `instruction_select.rs` | **Maximal Munch**: folding de inmediatos de 12 bits en `addi`/`andi`/`ori`, strength reduction en shifts. **Partición de constantes**: `lui` + `addi` con compensación de signo (bit 11) |
| 2 | `optimizer.rs` | **Strength reduction**: `mul`/`div` por potencia de 2 → `slli`/corrección signed. **Memory LVN**: elimina loads redundantes, hoista cálculos de dirección |
| 3 | `regalloc.rs` | **Linear Scan**: efímeras → `t0`–`t6`, call-crossing → `s1`–`s11`. Frame alineado a 16 bytes |
| 4 | — | **Prólogo/Epílogo ABI**: save/restore de `ra` + `sN`, manejo de frames > 2047 |
| 5 | `peephole.rs` | **Ventana deslizante**: elimina sd→ld redundante, addi 0→mv, `mv rd,rd`, folds consecutivos de sp. **RVC compression**: `c.add`, `c.mv`, `c.li`, `c.ld`/`c.sd`, `c.j`, `c.jr` (registros x8–x15) |

---

## 4. Pipeline Unificado (`src/pipeline.rs`)

```rust
pub fn run_pipeline(input: &str, src_path: &PathBuf, mode: BuildMode, arch: TargetArch)
    -> (PipelineResult, DiagnosticBag)
```

- **Full:** frontend → middle-end → borrow check → backend → `output.s` + binario
- **Check:** frontend → middle-end → borrow check → JSON diagnóstico a stdout
- **DumpAst:** frontend → JSON AST envuelto a stdout

## 5. Modo IDE

Cuando se usa `--check` o `--dump-ast`, el compilador emite **únicamente JSON estructurado a stdout**, sin mensajes humanos a stderr:

```json
{
  "success": true,
  "diagnostics": [
    { "phase": "parser", "severity": "error",
      "message": "...", "location": { "file": "...", "line": 5, "column": 3 } }
  ]
}
```

Para `--dump-ast`:
```json
{ "ast": { "nodeType": "program", "children": [...] } }
```

## 6. Decisiones Técnicas

1. **Sin Visitor pattern:** `TypedExpr` es un enum cerrado de 17 variantes. Usamos `match` directo en cada pase en vez de un Visitor trait — más simple, menos boilerplate, mismo resultado.

2. **Estructura inline-directory:** Cada módulo raíz es un `.rs` (ej. `arm64.rs`) y sus submódulos van dentro de `arm64/`. Sin `mod.rs`.

3. **IR tipo-string en codgen:** Las instrucciones IR usan `String` para nombres de registros/valores en vez de índices. Esto simplifica el backend a costa de una pequeña sobrecarga de allocación.

4. **Registros x0–x28 en ARM64:** `x29` (FP) y `x30` (LR) solo se reservan cuando hay llamadas a función. Si no las hay, están disponibles para el allocator.

5. **Asignación por pools en RISC-V:** Los valores que cruzan llamadas van a `s1–s11` (callee-saved). Los efímeros van a `t0–t6`. Esto evita save/restore innecesario.

6. **If-conversion en ARM64:** Solo aplica si los dos targets del branch son bloques sin efectos secundarios que convergen en un `Phi`. Caso contrario, se emite branch estándar.
