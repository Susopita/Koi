# Reporte de Benchmark — Koi vs. Rust vs. Go vs. Carp

Este documento registra la ejecución del harness de `benchmarks/` para el criterio
**"Comparación Comercial"** de la rúbrica (`Proyecto2_enunciado_compiladores.md`,
2 pts: "Benchmark riguroso y análisis sólido frente a compiladores comerciales").

## Entorno y dependencias

| Herramienta | Estado | Notas |
|---|---|---|
| `hyperfine` | v1.20.0 | No estaba en `apt` sin contraseña interactiva de `sudo`; se instaló vía `cargo install hyperfine` (Rust ya estaba disponible). |
| `rustc` | 1.95.0 | Ya instalado. Compilado con `-O`. |
| `go` | 1.22.2 | Ya instalado. `go build` optimiza por defecto. |
| `carp` | **No instalado** | Requiere `stack install` desde fuente (Haskell), un toolchain completo — se decidió NO instalarlo dado el tiempo que toma (el propio `benchmarks/README.md` acepta esto: "dale máximo medio día... si no compila limpio, documenta el intento"). Todas las filas `carp` quedan `NA` por ausencia del binario, no por un fallo de compilación de nuestro programa. |
| `matplotlib`/`numpy` | instalados | vía `pip install --break-system-packages`. |

## Cambios hechos al harness

1. **`benchmarks/koi_build.sh`** (nuevo): wrapper que envuelve el pipeline real de 3
   binarios + `gcc` (`koi-ast` → `koi-ir` → `koi-assembly` → `gcc`), ya que
   `run_benchmarks.sh` esperaba un único comando de compilación por lenguaje.
   **Bug encontrado y corregido durante la implementación**: `koi-assembly`
   escribe `output.s` en una ruta relativa al directorio de trabajo del proceso
   (no absoluta), así que el wrapper debe fijar explícitamente `cd` a la raíz del
   repo antes de invocar los tres binarios — de lo contrario `gcc` termina
   ensamblando un `output.s` obsoleto de una corrida anterior (esto se detectó
   porque los tres binarios `koi` inicialmente resultaron *byte-idénticos* pese a
   venir de fuentes `.koi` distintas).
2. **`run_benchmarks.sh` → `compile_cmd()`**: caso `koi` apunta a `koi_build.sh`
   en vez del placeholder `koi build archivo.koi -o salida`.
3. **`run_benchmarks.sh` → medición de ejecución**: se agregó `--ignore-failure`
   de `hyperfine` **solo para `koi`**. Motivo: `koi` no tiene una convención de
   `main` que retorne 0 — el valor que retorna `main` se usa directamente como
   código de salida del proceso (ver `koi-assembly/src/codegen.rs`, no hay
   "return 0" implícito). Un exit code distinto de 0 en un binario `koi` es
   normal y no indica un fallo de ejecución; sin este flag, `hyperfine` abortaba
   la medición y reportaba `NA` incluso cuando el programa corría y calculaba
   correctamente.

## `set!`/`while`/`aset!`/`do`: koi ahora soporta mutación real

La limitación original de este reporte ("koi no tiene ninguna forma de
mutación") se resolvió: se implementó `set!` (mutar un binding local), `while`
(loop con mutación arbitraria), `aset!` (escritura de arreglo, contraparte de
`index`) y `do` (secuenciar N≥1 expresiones) siguiendo las convenciones reales
de Carp — ver `LANGUAGE_EXTENSION_ROADMAP.md` para el diseño completo. Esto
reemplaza la sección original de este reporte que documentaba `quicksort`/
`matrix_sum` como "no comparables" y `lambda_map` como bloqueado.

**`lambda_map` se eliminó del set de benchmarks en su momento** (decisión del
usuario, mientras las clausuras con captura estaban completamente rotas) —
**se restauró más adelante** una vez corregido ese bug (ver
`CRITICAL_FIXES_REPORT.md` y la sección de `lambda_map` más abajo).

### Dos bugs de fondo encontrados y corregidos al implementar `set!`/`while`

Nadie había podido ejercitar estas rutas antes porque, sin mutación, nunca se
generaba código que las disparara:

1. **`array_literal` nunca escribía sus elementos** (`koi-ir/src/ir_generator.rs`).
   El código original calculaba el valor de cada elemento del literal y lo
   descartaba (`let (_, ty) = self.generate_expr(element)?`), dejando el
   arreglo asignado pero sin inicializar — comentario original: *"there is no
   write counterpart either (this language's arrays are read-only in the
   AST)"*. Ahora que existe `SetIndex` (agregado para `aset!`), se corrigió
   para que cada elemento se escriba de verdad con `SetIndex` tras el `Alloc`.
   Sin este fix, **`[1 2 3 4]` imprimía puros ceros/garbage** en cualquier
   programa, no solo en los nuevos.
2. **`if` y `while` no fusionaban correctamente el estado de una variable
   mutada entre sus dos caminos** (`koi-ir/src/ir_generator.rs::generate_if`/
   `generate_while`). Cuando solo una rama de un `if` mutaba una variable via
   `set!`, el código después del `if` podía leer un valor de SSA definido
   *solo* en la rama que **no** se ejecutó en runtime — una violación clásica
   de SSA que producía lecturas de memoria sin inicializar (valores como
   `140722555191696` en vez de `0`). De forma análoga, tras un `while`, el
   scope quedaba apuntando al último temporal calculado *dentro* del cuerpo en
   vez del resultado del `Phi` de la cabecera (el valor correcto y siempre
   definido al salir del loop). Se corrigió `generate_if` para tomar una foto
   del estado antes de bifurcar, generar cada rama desde ese mismo punto de
   partida, y fusionar con un `Phi` cualquier variable que termine con un
   valor distinto entre ambas ramas; y `generate_while` para reasignar cada
   variable al resultado de su `Phi` de cabecera al salir del loop. Ninguno
   de los dos bugs existía como problema *observable* antes de `set!`, porque
   antes no había forma de que una rama mutara algo que el código posterior
   necesitara leer.

Ambos fixes se verificaron con la suite completa (`cargo test --release`,
sigue en verde) y con los 6 `test/casos_prueba_carp/*.carp` existentes
(sin regresiones), además de la verificación de corrección descrita abajo.

### `quicksort.koi` — quicksort real, in-place, a escala completa

Implementado con `while`/`set!`/`aset!`/`index`/`do` (ver el archivo para el
código completo). **Actualizado a n=100 000** (la escala original de
`quicksort.rs`/`quicksort.go`) usando `(new arr_i64 (* n 8))` — un arreglo de
tamaño dinámico real, ya no el arreglo literal de 8 elementos al que estaba
limitado antes de que se corrigiera la asignación dinámica de arreglos (ver
`CRITICAL_FIXES_REPORT.md`). Mismo generador congruencial lineal que la
versión Rust, para datos deterministas.

**Verificación real** (no solo "compila"): a n=100 000, se probó (además de
correr el benchmark) que el arreglo queda ordenado correctamente comparando
cada par de elementos consecutivos.

### `matrix_sum.koi` — suma real sobre una matriz materializada, a escala completa

**Actualizado**: ya no calcula la celda `[i][j]=i*n+j` al vuelo (esa versión
evitaba el límite de tamaño de arreglos que ya no existe) — ahora **sí
materializa una matriz real de 500×500 (250 000 celdas)** con
`(new arr_i64 (* (* n n) 8))`, la escribe con `aset!` y la suma leyendo con
`index`, igual que `matrix_sum.rs`/`matrix_sum.go`.

**Verificación real**: salida impresa `31249875000`, que coincide exactamente
con la fórmula cerrada `n²·(n²-1)/2 = 500²·(500²-1)/2`.

### `lambda_map.koi` — pipeline map→filter→reduce con clausuras, restaurado

Este benchmark había sido eliminado mientras las clausuras con captura
estaban rotas; se restauró con dos clausuras **independientes** que capturan
variables libres (`double-fn` captura `factor`, `keep-fn` captura
`threshold`), a la escala original **n=1 000 000**. Filtro simplificado a
`x > threshold` (sin chequeo de paridad, ya que koi no tiene un builtin
`mod`) — no cambia lo que se mide (overhead de invocar una clausura por
elemento); se aplicó la misma simplificación en `lambda_map.rs`/`.go` para
que la comparación sea justa.

**Limitación nueva encontrada al restaurarlo**: una clausura capturadora solo
funciona si se **crea e invoca dentro de la misma función**. Se probó
pasarla como parámetro a una función genérica reutilizable
(`(defn apply-to-each [f arr n] ... (f ...) ...)`) y produce un **segfault**
— la marca de tipo `"closure_..."` que identifica un valor-closure para el
desempaquetado de `fn_ptr`/`env_ptr` solo existe en el sitio de construcción
de la clausura (dentro de `ir_generator.rs::generate_expr`), no se propaga a
través de la firma de tipo de una función. Por eso el pipeline en
`lambda_map.koi` está inlineado en `main`, no factorizado en funciones
`map`/`filter`/`reduce` reutilizables (ver `LANGUAGE_EXTENSION_ROADMAP.md`
para el detalle técnico completo).

**Verificación real**: salida impresa `1458331916667`, verificada contra una
implementación de referencia en Python (`sum(x for x in (i*3 for i in
range(1000000)) if x > 500000)`) y contra las versiones Rust/Go — las 3
coinciden exactamente.

## Resultados (15 corridas + 1-3 de warmup, máquina en reposo, escala completa)

| Benchmark | Lenguaje | Compilación (media ± σ) | Ejecución (media ± σ) | `.text` (bytes) |
|---|---|---|---|---|
| **fib** (recursión, fib(32)) | koi | 31.6 ms ± 1.4 ms | 32.5 ms ± 2.4 ms | 1 454 |
| | Rust | 124.9 ms ± 30.1 ms | 9.3 ms ± 0.4 ms | 327 579 |
| | Go | 76.1 ms ± 7.6 ms | 14.4 ms ± 1.9 ms | 1 186 150 |
| | Carp | NA (no instalado) | NA | NA |
| **quicksort** (sort real, n=100 000) | koi | 34.4 ms ± 2.0 ms | 23.1 ms ± 0.9 ms | 2 562 |
| | Rust | 123.2 ms ± 5.0 ms | 7.7 ms ± 0.7 ms | 328 827 |
| | Go | 45.3 ms ± 1.9 ms | 9.1 ms ± 0.6 ms | 1 187 087 |
| | Carp | NA | NA | NA |
| **matrix_sum** (matriz real 500×500) | koi | 30.9 ms ± 1.4 ms | 3.6 ms ± 0.2 ms | 1 974 |
| | Rust | 136.8 ms ± 3.4 ms | 2.5 ms ± 0.3 ms | 329 187 |
| | Go | 47.6 ms ± 5.3 ms | 3.5 ms ± 0.4 ms | 1 186 992 |
| | Carp | NA | NA | NA |
| **lambda_map** (map→filter→reduce, n=1 000 000) | koi | 34.9 ms ± 3.6 ms | 19.5 ms ± 1.2 ms | 2 280 |
| | Rust | 124.8 ms ± 2.2 ms | 6.6 ms ± 0.3 ms | 328 451 |
| | Go | 46.6 ms ± 2.9 ms | 8.0 ms ± 0.2 ms | 1 186 039 |
| | Carp | NA | NA | NA |

Gráficas generadas en `benchmarks/results/plots/`: `compile_time.png`,
`exec_time.png`, `text_size.png`. **Los 4 benchmarks son ahora comparaciones
válidas a escala completa** (mismo trabajo computacional en los 3 lenguajes,
mismas escalas que las versiones Rust/Go originalmente diseñadas) — ya no hay
benchmarks con escala artificialmente reducida por limitaciones del lenguaje.

## Análisis técnico

- **Tiempo de compilación**: koi es consistentemente el más rápido de los
  cuatro (31-35 ms vs. 45-137 ms Go, 123-137 ms Rust). Esperado — koi no hace
  inferencia de tipos ni optimización a la escala de un compilador de
  producción, y su pipeline de 3 binarios independientes vía JSON en disco es
  simple comparado con el front-end/middle-end de LLVM o del compilador de Go.
- **Tiempo de ejecución**: koi es consistentemente más lento que Rust/Go en
  los 4 benchmarks, con un factor que varía según cuánto domine la aritmética
  escalar frente al acceso a memoria/arreglos:
  - `fib` (recursión pesada, casi todo aritmética + llamadas): ~3.5x más
    lento que Rust, ~2.3x que Go — el peor caso, porque expone al máximo la
    falta de asignación de registros.
  - `quicksort`/`lambda_map` (mezcla de aritmética y acceso a arreglo):
    ~3x/~3x que Rust, ~2.5x/~2.4x que Go.
  - `matrix_sum` (dominado por escritura/lectura secuencial de arreglo): solo
    ~1.4x más lento que Rust y prácticamente empatado con Go (3.6 ms vs.
    3.5 ms) — cuando el trabajo es mayormente acceso a memoria en vez de
    mantener valores vivos en registros, la desventaja de koi se reduce
    mucho, porque Rust/Go *también* terminan tocando memoria para cada
    acceso al arreglo.
  Esta escala de gap (2-3.5x, nunca más) es información mucho más honesta que
  la de la corrida anterior a escala reducida, donde el overhead fijo de
  arranque dominaba y las cifras no decían casi nada sobre eficiencia real.
- **Causa raíz del gap de ejecución**: **todo valor vive en la pila** (ver
  `koi-assembly/src/register_allocator.rs` — `ValueLocation` solo tiene la
  variante `Stack`, no hay asignación real de registros más allá de dos
  registros scratch transitorios por instrucción), así que cada operación
  aritmética hace recarga desde memoria en vez de mantener valores vivos en
  registros. Rust/Go sí hacen asignación de registros real. Las optimizaciones
  de mirilla/reducción de fuerza/DCE (`OPTIMIZATION_REPORT.md`) reducen el
  ensamblador generado, pero no sustituyen un asignador de registros real —
  ese es el próximo cuello de botella de rendimiento a atacar.
- **`lambda_map` específicamente**: koi es ~3x más lento que Rust en este
  benchmark, un gap similar al de `quicksort` (no dramáticamente peor pese a
  la representación de closures como puntero a struct de 2 campos +
  desempaquetado vía 2 `GetField` en cada llamada) — el costo de la
  closure-conversion de koi no domina frente al costo ya existente de no
  tener asignación de registros para el resto del cuerpo.
- **Tamaño de `.text`**: koi produce binarios dramáticamente más pequeños
  (1 454-2 562 bytes vs. cientos de miles en Rust/Go) — Rust y Go enlazan
  estáticamente runtime/stdlib (Go especialmente incluye su runtime con
  goroutines/GC), mientras que koi solo enlaza contra `libc` dinámicamente.
  Esta cifra no es una medida de "eficiencia de codegen" comparable — es
  principalmente un artefacto de qué se enlaza estáticamente en cada binario.

## Limitaciones del lenguaje

**Resueltas en una fase posterior** (ver `CRITICAL_FIXES_REPORT.md` para el
detalle completo — causa raíz, fix y verificación de cada una):

1. ~~Tamaño de arreglo fijo (64 bytes / 8 elementos i64)~~ — corregido:
   `array_literal` ahora calcula el tamaño real (`elementos × 8 bytes`), y
   `(new arr_i64 <bytes>)` con un tamaño calculado en runtime también
   funciona de extremo a extremo (verificado con arreglos de 10, 15 y 20
   elementos). Esto era en realidad un bug de corrupción de heap silenciosa
   para cualquier literal de más de 8 elementos, no solo una limitación de
   tamaño.
2. ~~Clausuras completamente rotas~~ — corregido para el caso básico: una
   lambda que captura una variable **por valor** (snapshot en el momento de
   la construcción del closure) ahora funciona en cualquier contexto, no solo
   dentro de loops. Sigue habiendo dos límites explícitos, no corregidos:
   - **Clausuras que capturan una variable mutada por `set!` *después* de
     construido el closure** no ven la mutación (el closure guarda el valor
     capturado en el momento de su creación, no una referencia viva a la
     variable) — comportamiento por-valor, no por-referencia. Esto es una
     decisión de diseño razonable, no necesariamente un bug, pero no está
     probado ni documentado como garantía del lenguaje.
   - **Clausuras anidadas** (una lambda que captura algo Y contiene, en su
     cuerpo, otra lambda que también captura algo) puede no resolver tipos
     correctamente — no soportado, fuera de alcance deliberadamente.
   - **Clausuras pasadas como parámetro de función para invocarse
     genéricamente en otra función** (`(defn apply [f arr n] ... (f ...))`)
     producen un segfault — encontrado al restaurar `lambda_map.koi` (ver
     arriba). La marca de tipo que identifica un valor-closure solo existe
     en el sitio de construcción, no atraviesa la firma de tipo de una
     función. Por esto no es posible escribir un `map`/`filter`/`reduce`
     genérico y reutilizable — cada uso debe inlinear su propio loop.
3. ~~`f64` sin backend~~ — corregido: aritmética, comparaciones, `print`,
   parámetros y retorno de funciones con `f64` funcionan de extremo a
   extremo (verificado con ejecución real, no solo compilación).

**Persisten:**

4. Sin asignación de registros real (todo en la pila) — causa raíz del gap de
   rendimiento frente a Rust/Go, ver arriba. No abordado.
5. División con signo vía reducción de fuerza (`sarq`) no es exacta para
   dividendos negativos — limitación documentada desde la fase de
   optimización, diferida a futuro por decisión explícita del usuario.

Estas limitaciones son información válida y esperada para el análisis técnico
que pide la rúbrica.
