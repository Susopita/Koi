# Mutación al estilo Carp en koi: `set!` / `while` / `aset!` / `do` — IMPLEMENTADO

Este documento originalmente respondía a la pregunta: *siguiendo las convenciones de
Carp, ¿cómo se extendería koi para soportar `quicksort`/`matrix_sum` de verdad?* —
se dejó como ruta documentada, sin implementar, tras no recibir respuesta del usuario
a la pregunta de si proceder. El usuario luego confirmó proceder (eliminando
`lambda_map` del alcance), y la implementación se completó. El diseño original se
conserva abajo como documentación de las decisiones tomadas; ver
`benchmarks/BENCHMARK_REPORT.md` para los resultados finales, la verificación de
corrección de `quicksort`/`matrix_sum`, y dos bugs de fondo encontrados y corregidos
durante la implementación (que no eran parte del diseño original):

1. `array_literal` nunca escribía sus elementos en el arreglo asignado (bug
   preexistente, invisible hasta que existió una instrucción de escritura que
   pudiera arreglarlo).
2. `if`/`while` no fusionaban correctamente el estado de una variable mutada
   entre sus dos caminos (violación de SSA, invisible hasta que existió `set!`).

**Nota**: en una fase posterior (no relacionada con `set!`/`while`), se corrigieron
además 3 mejoras críticas distintas — backend de `f64`, tamaño real de arreglos
dinámicos (usando el mismo mecanismo de `SetIndex` introducido aquí), y clausuras
con captura de variables — ver `CRITICAL_FIXES_REPORT.md`.

## Desviaciones respecto al plan original

- **`lambda_map` fuera de alcance** (decisión del usuario): no se implementó el
  punto 5 (preludio `Array.map`/`filter`/`reduce`) ni el punto 4 (constructores de
  struct), ya que solo eran necesarios para ese benchmark.
- **`aget` no se agregó como builtin**: se confirmó que reusar el nodo `Index`
  existente (ya funcional) era preferible — sin duplicar lógica para el mismo
  fin. Solo `aset!` es nuevo.
- **`generate_loop`/`patch_loop_phi` (mecanismo de `loop`) se dejaron intactos**,
  tal como estaba planeado — `while` tiene su propio mecanismo, más general.
- El plan original no anticipó los dos bugs de fondo (`array_literal` y
  `if`/`while` merge) — surgieron al verificar `quicksort`/`matrix_sum` de
  extremo a extremo (no solo compilación), y se corrigieron como parte de esta
  misma tarea al ser condición necesaria para que `set!`/`while` funcionaran
  correctamente en cualquier caso no trivial.

## Por qué hace falta (resumen de `BENCHMARK_REPORT.md`)

koi no tiene ninguna forma de mutación: sin nodo de asignación en el AST
(`koi-ast/src/ast.rs`), sin instrucción de escritura en memoria en el IR
(`koi-assembly/src/ir_parser.rs` solo tiene `GetField`/`GetIndex` de lectura), y
`loop` retorna la variable de control, no el cuerpo (`koi-ir/src/ir_generator.rs`).
Por eso `quicksort`/`matrix_sum` no pueden implementarse de verdad (in-place swap,
acumulación sobre un arreglo) y `map`/`filter`/`reduce` no existen.

## Piezas mínimas, siguiendo convenciones reales de Carp

Carp mismo no inventa mutación arbitraria: usa un sistema de ownership donde un
valor tiene un único dueño en cada momento, lo que permite que operaciones como
`aset!` hagan una escritura real in-place y aun así se comporten como si
"devolvieran un nuevo valor" (no hay aliasing que romper). Replicar exactamente el
borrow-checker de Carp está fuera de alcance de este proyecto (MVP), pero se pueden
tomar sus formas de superficie sin el chequeo de ownership completo — igual que koi
ya simplifica otras partes de Carp.

### 1. `set!` — mutación de un binding local

- **AST** (`koi-ast/src/ast.rs`): nuevo variante `SetVar { name: String, value: Box<ASTNode>, line, column }`.
- **Parser** (`koi-ast/src/parser.rs`): nueva `parse_set!` análoga a `parse_let`
  (`(set! nombre valor)`), registrada en el dispatch de `parse_expr` junto a `if`/`let`/`loop`.
- **IR** (`koi-ir/src/ir_generator.rs`): romper el supuesto de que un nombre de
  variable local mapea a un único valor SSA fijo — mantener, por bloque, cuál es
  la definición SSA "actual" de cada nombre mutable (patrón estándar de
  construcción SSA con Phi, el mismo mecanismo que `generate_loop` ya usa para
  *una* variable, generalizado a *cualquier* variable mutable en scope).

### 2. `aset!`/`aget` — lectura/escritura real de arreglos

- **Sin nodo AST nuevo**: tratarlos como llamadas normales (`Call` con función
  `"aset!"`/`"aget"`), igual que `print`/`malloc`/`free` hoy.
- **`koi-ir/src/builtins.rs`**: agregar `"aset!"` y `"aget"` a `BuiltinKind`/`BUILTIN_NAMES`
  (o reusar el nodo `Index` ya existente para `aget`, que ya funciona; solo falta el
  lado de escritura).
- **IR** (`koi-assembly/src/ir_parser.rs`): nueva variante `Instruction::SetIndex { array, index, value, type }`,
  simétrica a `GetIndex` que ya existe.
- **Codegen** (`koi-assembly/src/codegen.rs`): `emit_get_index` ya calcula
  `array_ptr + index*element_size`; `emit_set_index` reutiliza exactamente ese
  cálculo de dirección y hace `movq value_reg, 0(%scratch)` en vez de leer.

### 3. `while` — iteración con mutación arbitraria

- **AST**: nuevo variante `WhileExpr { condition: Box<ASTNode>, body: Box<ASTNode>, line, column }`.
- **Parser**: `parse_while`, análoga a `parse_if`.
- **IR**: lowering análogo a `generate_loop` pero sin una única "variable" fija —
  el cuerpo puede contener cualquier número de `set!`, cada uno generando su
  propio Phi en el header del loop (la parte de "reconciliar valores de vuelta al
  header" que `patch_loop_phi` ya hace para una variable se generaliza a N).

### 4. (Opcional, más fiel a Carp) Constructores de struct como función

Carp genera automáticamente una función constructora por cada `defstruct`
(`(defstruct Vector2 [x Float] [y Float])` → `(Vector2 3.0 4.0)` construye una
instancia). koi hoy solo tiene `new TypeName` (alloc sin inicializar campos).
Agregar esto requeriría: reconocer en `koi-ast` cuando el símbolo en posición de
función de un `Call` coincide con un `struct_def` conocido, y en `koi-ir` bajarlo a
`Alloc` + N `SetField` (instrucción nueva, simétrica a `GetField` igual que
`SetIndex` lo es a `GetIndex`). **No es necesario para quicksort/matrix_sum**
(ambos solo necesitan arreglos planos), solo sería necesario si se quisiera una
implementación de `lambda_map` fiel a un `Array.map` genérico sobre structs.

### 5. `Array.map`/`Array.filter`/`Array.reduce` — como preludio, no como builtins

Forma más fiel a Carp: estas NO son intrínsecos del compilador en Carp real, están
escritas en la propia librería estándar de Carp usando `while`/`aset!`. Una vez
existan (1) y (3), se pueden escribir como funciones normales de koi:

```lisp
(defn array-reduce [f init arr n]
  (let [acc init i 0]
    (do
      (while (< i n)
        (do
          (set! acc (f acc (aget arr i)))
          (set! i (+ i 1))))
      acc)))
```
...e inyectarse como un "preludio" (`prelude.carp`) que `koi-ast` parsea y concatena
antes del programa del usuario — análogo a cómo Carp inyecta su `core.carp`.

## Alcance por caso de benchmark (estado final)

| Benchmark | Piezas usadas | Estado |
|---|---|---|
| `quicksort` | (1) `set!`, (2) `aset!`+`index`, (3) `while` | ✅ implementado, sort real verificado |
| `matrix_sum` | (1) `set!`, (3) `while` (sin arreglo, acumulador escalar) | ✅ implementado, suma real verificada contra fórmula cerrada |
| `lambda_map` | (5) preludio + fix del bug de clausuras en loops | ❌ fuera de alcance (decisión del usuario), eliminado de `benchmarks/` |

## Equipo de agentes usado (5, tal como se planeó)

1. koi-ast (`SetVar`/`WhileExpr`/`DoExpr` + parser + scope) — en paralelo con 2 y 4.
2. koi-ir tipos/inferencia/builtins (`Type::Unit`, `BuiltinKind::SetIndex`,
   `inference.rs`) — en paralelo con 1 y 4.
3. koi-ir generación de código (`ir_generator.rs`/`lambda_lifter.rs`,
   `generate_while`/`reassign`) — después del 2, mismo crate.
4. koi-assembly (`Instruction::SetIndex`, `emit_set_index`, DCE/register
   allocator) — en paralelo con 1 y 2.
5. Integración final + benchmarks reales — falló a mitad de camino por un
   límite de sesión de la API (no relacionado con el código); se retomó y
   completó manualmente, incluyendo el descubrimiento y arreglo de los dos
   bugs de fondo descritos arriba (no anticipados por ningún agente, ya que
   solo se manifiestan al *ejecutar* `quicksort`/`matrix_sum` de extremo a
   extremo, no al compilarlos).
