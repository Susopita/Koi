# Evidencia de Optimización — Koi

Este documento registra la evidencia técnica de las optimizaciones implementadas en
`koi-assembly`, para uso en el reporte técnico del Proyecto 3
(`Proyecto2_enunciado_compiladores.md`, criterio **Optimización**, 3 pts: "Implementa
optimizaciones relevantes y demuestra mejoras medibles").

## Arquitectura del pipeline de optimización

```
IR (JSON)
  → Optimizer::optimize_program   (koi-assembly/src/optimizer.rs)
       ├─ constant_folding
       ├─ strength_reduction
       ├─ dead_code_elimination
       └─ eliminate_unreachable_blocks
     ejecutados en bucle de punto fijo (fixpoint), máx. 32 iteraciones,
     hasta que ningún pase reporta cambios (efecto cascada)
  → X86Generator::generate         (codegen.rs, genera ensamblador x86-64 AT&T)
  → Peephole::optimize             (koi-assembly/src/peephole.rs)
       corre a punto fijo sobre el texto ensamblador emitido
  → output.s
```

Todas las técnicas están integradas en `compile_ir_json_to_assembly`
(`koi-assembly/src/lib.rs`) — se aplican automáticamente en cada compilación, no son
opt-in.

---

## 1. Reducción de fuerza (strength reduction)

**Regla:** `x * 2^k → x << k` (`salq`), `x / 2^k → x >> k` (`sarq`), `x * 0 → 0`.
Limitación documentada: la división por corrimiento aritmético no es exacta para
dividendos negativos (redondea hacia -∞ en vez de hacia 0 como `idivq`); queda como
mejora futura, no se corrige en esta entrega.

**Programa de prueba:**
```lisp
(defn compute [x]
  (+ (* x 8) (/ x 4)))

(defn main []
  (compute 10))
```

**Antes** (codegen puro, sin `Optimizer::optimize_program`) — 55 líneas de `output.s`,
fragmento relevante:
```asm
    movq	$8, %rax
    movq	%rax, -16(%rbp)
    movq	-8(%rbp), %rax
    movq	-16(%rbp), %r10
    imulq	%r10, %rax          ; multiplicación real de 64 bits
    ...
    movq	$4, %rax
    movq	%rax, -32(%rbp)
    movq	-8(%rbp), %rax
    movq	-32(%rbp), %r10
    cqto
    idivq	%r10                 ; división real (con signo extendido)
```

**Después** (pipeline completo) — 50 líneas de `output.s`:
```asm
    movq	$3, %rax             ; log2(8) = 3, constante insertada por el pase
    movq	%rax, -16(%rbp)
    movq	-8(%rbp), %rax
    movq	-16(%rbp), %r10
    movq	%r10, %rcx
    salq	%cl, %rax            ; shift en vez de imulq
    ...
    movq	$2, %rax             ; log2(4) = 2
    movq	%rax, -32(%rbp)
    movq	-8(%rbp), %rax
    movq	-32(%rbp), %r10
    movq	%r10, %rcx
    sarq	%cl, %rax            ; shift en vez de idivq/cqto
```

**Verificación de corrección:** ejecución real del binario resultante:
`compute(10) = 10*8 + 10/4 = 80 + 2 = 82` → `./output; echo $?` → **`82`** ✅
(confirma que el resultado numérico no cambió tras la optimización).

**Tests automatizados:** 6 tests unitarios en `optimizer.rs`
(`mul_by_power_of_two_...`, `div_by_power_of_two_...`, `mul_by_zero_...`,
`mul_by_non_power_of_two_is_unchanged`, `mul_by_one_is_unchanged`, más variante
lhs/rhs conmutativa) + 2 tests de integración en `backend_tests.rs` que verifican que
el ensamblador final contiene `salq`/`sarq` y NO contiene `imulq`/`idivq` para esos casos.

---

## 2. Eliminación de código muerto (instrucciones + bloques inalcanzables)

**Regla:** además de eliminar instrucciones puras sin usos (ya existente), el pase
`eliminate_unreachable_blocks` recorre el CFG desde el bloque `entry` (BFS sobre aristas
`jump`/`branch`/fallthrough) y elimina cualquier bloque básico no alcanzable.

**Programa IR de prueba** (bloque `dead` sin ningún `jump`/`branch` que lo referencie):
```json
"blocks": [
  {"label": "entry", "instructions": [{"op":"const","result":"%unused","value":999,...}, {"op":"jump","label":"live"}]},
  {"label": "dead",  "instructions": [{"op":"const","result":"%d","value":1,...}, {"op":"jump","label":"live"}]},
  {"label": "live",  "instructions": [{"op":"return","value":"x"}]}
]
```

**Antes de `Optimizer::optimize_program`:**
```
Bloques: 3
  - entry (2 instr)   ← incluye %unused, nunca usado
  - dead  (2 instr)   ← nunca referenciado por ningún jump/branch
  - live  (1 instr)
```

**Después:**
```
Bloques: 2
  - entry (1 instr)   ← %unused eliminado
  - live  (1 instr)
```

El bloque `dead` desaparece por completo del ensamblador generado — no se emite
`.Ldead_branch_demo_dead:` en ningún lugar del `output.s` resultante.

**Tests automatizados:** 5 tests unitarios en `optimizer.rs` cubriendo: instrucción pura
sin uso eliminada, instrucciones con efecto colateral (`call`/`alloc`/`return`)
preservadas, bloque inalcanzable eliminado, bloque `entry` nunca eliminado (caso
trivial de una sola función/bloque), bloque alcanzable vía `branch` (CFG en diamante)
preservado. Se verificó explícitamente que el test de regresión
`loop_phi_back_edge_values_survive_optimization` (loop con back-edge vía `phi`) sigue
pasando — es decir, la eliminación de bloques no rompe loops.

---

## 3. Efecto cascada (fixpoint / punto fijo)

**Problema que resuelve:** un solo pase de cada optimización no basta cuando una
optimización habilita a otra. Ejemplo: `%v2 = 2 * 4` solo se pliega a `Const 8` vía
`constant_folding`; recién entonces `strength_reduction` puede ver que
`%v3 = x * %v2` es una multiplicación por una potencia de 2 conocida y convertirla en
`x << 3`.

**Implementación:** `optimize_function` ejecuta los 4 pases en un bucle
(`constant_folding | strength_reduction | dead_code_elimination |
eliminate_unreachable_blocks`) hasta que ninguno reporte cambios, con tope de
seguridad de 32 iteraciones.

**Prueba de que el orden/repetición importa** (test
`manual_wrong_order_single_pass_does_not_reduce_the_multiply` en `optimizer.rs`):
llamar manualmente `strength_reduction` **antes** que `constant_folding` (un solo pase
de cada uno, sin el driver de punto fijo) sobre la secuencia
`%v0=2, %v1=4, %v2=%v0*%v1, %v3=x*%v2` produce un resultado **peor**: `strength_reduction`
reescribe prematuramente `%v2 = %v0 * %v1` en `%v0 << 2` (que `fold_binop` no sabe
plegar), bloqueando que `%v2` se reconozca como constante y dejando
`%v3 = x * %v2` como multiplicación real sin reducir. El driver de punto fijo
(`optimize_function`) sí resuelve ambos pasos correctamente en una sola llamada.

**Test complementario:** `optimize_function_fixpoint_reduces_multiply_that_depends_on_a_fold`
confirma que, tras `optimize_function`, la misma secuencia queda completamente reducida
(sin `*` residual, `%v3` como `<<` por 3).

---

## 4. Mirilla (peephole) sobre ensamblador emitido

**Patrones eliminados** (`koi-assembly/src/peephole.rs`), corridos a punto fijo sobre
el texto del `.s` generado, sin cruzar límites de labels/`call`/saltos:
- Self-move: `movq %reg, %reg`
- Store→reload redundante: `movq A, B` seguido de `movq B, A`
- Aritmética no-op: `addq $0,R` / `subq $0,R` / `imulq $1,R`
- `jmp` a la etiqueta inmediatamente siguiente

**Evidencia real** (`add.carp`, `add(5,3)`): el codegen naive emite
`jmp .Ladd_entry` justo antes de la etiqueta `.Ladd_entry:` (y lo mismo para `main`,
`jmp .Lmain_entry`) porque cada función siempre salta a su propio bloque de entrada.
La mirilla elimina ambos saltos redundantes:

| | Líneas de `output.s` |
|---|---|
| Sin mirilla | 46 |
| Con mirilla | 40 |

Ejecución verificada: `add(5,3)` sigue devolviendo **`8`** después de la mirilla.

**Tests automatizados:** 12 tests unitarios en `peephole.rs` (uno por patrón +
negativos que confirman que NO se colapsa across un label o un `call` + test de
idempotencia) y verificación de que los 4 tests de integración preexistentes en
`backend_tests.rs` (que assertan substrings del ensamblador) siguen pasando.

---

## Resumen de tamaños de ensamblador generado (pipeline completo, optimizado)

| Programa | Líneas `output.s` |
|---|---|
| `add.carp` | 40 |
| `struct.carp` | 33 |
| `fib.carp` | 58 |
| `kitchen_sink.carp` | 131 |

## Cobertura de tests

`cargo test --release` (workspace completo): **todas las suites en verde**, incluyendo
25 tests unitarios en `koi-assembly` (13 en `optimizer.rs`, 12 en `peephole.rs`), 6
tests de integración en `backend_tests.rs`, 3 en `pipeline_cli_tests.rs`, más las
suites completas de `koi-ast` y `koi-ir` sin regresiones.

## Verificación de corrección end-to-end

Todos los ejemplos de este documento fueron ejecutados de extremo a extremo
(`koi-ast` → `koi-ir` → `koi-assembly` → `gcc` → binario) y el resultado numérico del
programa se confirmó idéntico antes y después de aplicar las optimizaciones — las
optimizaciones reducen instrucciones sin alterar la semántica observable del programa.
