# Koi Compiler — Interface Specification

> Documentación de las interfaces de entrada/salida del compilador koi
> para integración con otras herramientas (IDEs, CI/CD, pipelines).

---

## 1. CLI (Command Line Interface)

```
koi [FLAGS] <COMMAND>
```

### Subcomandos

```
build     Compilar un archivo .koi a binario o assembly
```

### Flags globales

| Flag | Descripción |
|---|---|
| `-h`, `--help` | Ayuda del comando |
| `-V`, `--version` | Versión del compilador |

### Flags de `build`

```
koi build [FLAGS] [OPTIONS] <FILE>
```

| Flag | Descripción |
|---|---|
| `--target <ARCH>` | Arquitectura destino: `arm64`, `x86_64`, `riscv` |
| `--no-optimize` | Salta optimizer, scheduler y peephole |
| `--check` | Solo type-check (sin generar binario) |
| `--dump-ast` | Imprime el AST como JSON a stdout y termina |
| `--output <FILE>` | Nombre del binario de salida |

### Exit codes

| Código | Significado |
|---|---|
| `0` | Compilación exitosa |
| `1` | Error de compilación (parse, tipo, scope, etc.) |
| `139` | Segfault en el binario generado (no del compilador) |
| >0 (koi build) | Exit code del programa compilado (no del compilador) |

### Salida estándar (stdout)

En modo `build` exitoso:
```
[koi] Build complete: output.s + executable '<name>'
```

En modo `--dump-ast`: el JSON del AST (ver sección 3).

En modo `--check`: sin salida si es exitoso.

### Salida de error (stderr)

Formato de errores:
```
[parser] <mensaje> at line N, column M
[scope] <mensaje> at line N, column M
[type] <mensaje> at line N, column M
[borrow_check] <mensaje>
[linker] warning: <mensaje>
```

Errores de parser/scope/type tienen ubicación exacta (line, column).
Errores de borrow_check pueden o no tener ubicación.

Ejemplo:
```
[parser] Expected ) to close let expression. Got LParen at line 6, column 5
[borrow_check] cannot mutably borrow 'p': status is MutBorrowed
[linker] warning: assembly failed (assembler not available on this host)
```

---

## 2. Modo `--dump-ast` — JSON AST

Cuando se pasa `--dump-ast`, el compilador imprime el AST como JSON
a stdout y termina. No se genera binario.

### Formato raíz

```json
{
  "tag": "program",
  "children": [
    { "tag": "function_def", ... },
    { "tag": "struct_def", ... },
    { "tag": "call", ... }
  ]
}
```

### Node types (tag values)

```
program           → Raíz del programa
function_def      → Definición de función
struct_def        → Definición de struct (defstruct / deftype)
call              → Llamada a función
variable          → Referencia a variable
literal           → Literal (int, float, bool, string)
lambda            → Lambda expression
let_binding       → Let binding
if                → If expression
loop              → Loop expression
while             → While loop
do                → Do block
field_access      → Acceso a campo de struct
set_field         → Escritura de campo de struct
index             → Index read (arr[i])
set               → set! (mutación de variable)
new               → Heap allocation
addr_of           → Address-of (&)
deref             → Dereference (*)
array_literal     → Array literal [a b c]
make_closure      → Constructor de closure (compiler-internal)
```

### Schema por node type

**function_def**
```json
{
  "tag": "function_def",
  "name": "fib",
  "parameters": [
    { "name": "n", "type": null }
  ],
  "body": { ... },
  "line": 1,
  "column": 1
}
```

`parameters[].type` es `null` si no se especificó tipo, o `"i64"` si
se usó `[n :i64]`.

**struct_def**
```json
{
  "tag": "struct_def",
  "name": "Point",
  "fields": [
    ["x", "i64"],
    ["y", "i64"]
  ],
  "line": 1,
  "column": 1
}
```

Cada field es `[nombre, tipo]`. `deftype` y `defstruct` producen
el mismo AST.

**call**
```json
{
  "tag": "call",
  "function": { "tag": "variable", "name": "+", ... },
  "arguments": [ ... ],
  "line": 1,
  "column": 1
}
```

**literal**
```json
// Entero
{ "tag": "literal", "literal_type": "int64", "value": 42, ... }

// Flotante
{ "tag": "literal", "literal_type": "float64", "value": 3.14, ... }

// Booleano
{ "tag": "literal", "literal_type": "bool", "value": true, ... }

// String
{ "tag": "literal", "literal_type": "string", "value": "hello", ... }
```

**lambda**
```json
{
  "tag": "lambda",
  "parameters": [
    { "name": "x", "type": null }
  ],
  "body": { ... },
  "line": 1,
  "column": 1
}
```

**let_binding**
```json
{
  "tag": "let_binding",
  "bindings": [
    ["x", { "tag": "literal", ... }],
    ["y", { "tag": "literal", ... }]
  ],
  "body": { ... },
  "line": 1,
  "column": 1
}
```

Bindings Carp-style: todos se evalúan en el scope exterior, luego
se registran simultáneamente.

**if**
```json
{
  "tag": "if",
  "condition": { ... },
  "then_branch": { ... },
  "else_branch": null,
  "line": 1,
  "column": 1
}
```

`else_branch` es `null` cuando no hay else.

**addr_of / deref**
```json
// Address-of
{ "tag": "addr_of", "operand": { ... }, ... }

// Dereference
{ "tag": "deref", "operand": { ... }, ... }
```

**make_closure** (solo en AST post-lambda-lifting)
```json
{
  "tag": "make_closure",
  "function_name": "_lambda_0",
  "captured": ["factor"],
  "line": 1,
  "column": 1
}
```

### Ejemplo completo

Input:
```clojure
(defn add [x y] (+ x y))
```

Output JSON:
```json
{
  "tag": "program",
  "children": [
    {
      "tag": "function_def",
      "name": "add",
      "parameters": [
        { "name": "x", "type": null },
        { "name": "y", "type": null }
      ],
      "body": {
        "tag": "call",
        "function": { "tag": "variable", "name": "+", "line": 1, "column": 23 },
        "arguments": [
          { "tag": "variable", "name": "x", "line": 1, "column": 25 },
          { "tag": "variable", "name": "y", "line": 1, "column": 28 }
        ],
        "line": 1,
        "column": 23
      },
      "line": 1,
      "column": 1
    }
  ]
}
```

---

## 3. Modo `--check` — Output de type-checking

En modo `--check` exitoso: stdout vacío, exit code 0.

En modo `--check` con error:

```json
{
  "ok": false,
  "diagnostics": [
    {
      "severity": "error",
      "message": "Undefined variable: 'foo' at line 3, column 7",
      "line": 3,
      "column": 7,
      "phase": "type"
    }
  ],
  "duration_ms": 12.5
}
```

> Nota: el output JSON en `--check` es la salida a implementar.
> Actualmente los errores se imprimen como texto plano en stderr.

---

## 4. Archivos generados en modo `build`

Al ejecutar `koi build --target arm64 archivo.koi`:

| Archivo | Contenido |
|---|---|
| `output.s` | Assembly textual (siempre generado) |
| `<archivo>` | Binario ejecutable (si el ensamblado+linkeo funciona) |

El nombre del binario es el nombre base del archivo fuente sin extensión.
Si el source es `fib.koi`, el binario es `./fib`.

El assembly generado depende del target:

- **ARM64**: AArch64 assembly (`.arch armv8-a`). En macOS usa
  `__TEXT,__const` para datos, en Linux usa `.section .rodata`.
- **x86_64**: AT&T syntax assembly. Requiere GCC/Clang para ensamblar.
- **RISC-V**: RV64 assembly. Opcionalmente comprimido (RVC) si
  el peephole optimizer está activo.

### Secciones del assembly

```
.arch armv8-a                     ; target architecture
.section __TEXT,__const           ; read-only data (macOS) / .rodata (Linux)
.LC_print_i64: .asciz "%ld\n"     ; format strings for print()
.LC_print_string: .asciz "%s\n"
.LC_print_f64: .asciz "%f\n"
.text                             ; code section
.balign 4
.globl main                       ; entry point
main:
  stp x29, x30, [sp, #-16]!       ; prologue
  ...
  ret                             ; return
```

### Convención de llamada

En ARM64 (macOS): Apple ARM64 ABI.
- Argumentos: x0-x7, luego stack
- Variadic (printf): argumentos extra en stack
- Callee-saved: x19-x28, x29 (fp), x30 (lr)
- Caller-saved: x0-x18

En ARM64 (Linux): AAPCS64 (misma ABI sin variadic en stack).

---

## 5. Pipeline Steps (para debug / integración)

El pipeline completo:

```
Source (.koi)
  │
  ▼
┌─────────────┐
│ Scanner     │ → Tokens
└─────────────┘
  │
  ▼
┌─────────────┐
│ Parser      │ → AST (JSON con --dump-ast)
└─────────────┘
  │
  ▼
┌─────────────┐
│ Scope       │ → AST anotado
└─────────────┘
  │
  ▼
┌─────────────┐
│ Inference   │ → Types + Constraints
└─────────────┘
  │
  ▼
┌─────────────┐
│ Lambda Lift │ → AST sin closures
└─────────────┘
  │
  ▼
┌─────────────┐
│ IR Gen      │ → SSA IR (Instrucciones)
└─────────────┘
  │
  ▼
┌─────────────┐
│ Optimizer   │ → IR optimizado (opcional)
└─────────────┘
  │
  ▼
┌─────────────┐
│ Backend     │ → Assembly (.s)
└─────────────┘
  │
  ▼
┌─────────────┐
│ Assembler   │ → Binario (vía clang/gcc)
└─────────────┘
  │
  ▼
Binary (ejecutable)
```

Cada paso puede fallar con errores en stderr. No hay salida
intermedia en JSON excepto `--dump-ast` que produce el AST
después del parsing.

---

## 6. Variables de entorno

| Variable | Efecto |
|---|---|
| `KOI_DEBUG=1` | Muestra stderr del assembler/linker |
| `KOI_FORCE_TARGET` | Override de target (`arm64`, `x86_64`, `riscv`) |

---

## 7. Dependencias externas

Para compilar el compilador: `cargo` + `rustc` (nix-shell recomendado).

Para ensamblar y linkear: `clang`, `cc` o `gcc` en PATH.
En macOS, `clang` viene incluido con Xcode Command Line Tools.
En Linux, `gcc` está disponible como package del sistema.

El compilador busca en orden: `clang` → `cc` → `gcc`.
En macOS, si `gcc --version` contiene "Apple" o "clang", se usa
ese binario (Apple Clang disfrazado de gcc).

---

## 8. Integración con editores / IDE

Para integrar koi con un editor:

1. **Syntax highlighting**: usar archivos `.koi` con sintaxis
   similar a Clojure/Carp (S-expressions, `;` comments,
   `defn`/`defstruct`/`deftype`/`lambda`/`fn` keywords).

2. **Diagnostics**: ejecutar `koi build --check archivo.koi`.
   Errores en stderr con formato `[fase] mensaje at line N, column M`.

3. **AST visualization**: `koi build --dump-ast archivo.koi`
   produce JSON del AST en stdout.

4. **Build**: `koi build --target arm64 archivo.koi` produce
   `output.s` + binario ejecutable.

5. **Exit code** 0 = éxito, 1 = error de compilación.
