# CS3402 Compiladores 2026-1

## Proyecto 3: Diseño e Implementación de un Compilador x86 con Optimización y Evaluación Comparativa

### Objetivo General

Diseñar e implementar un **compilador completo** para un lenguaje de programación sobre arquitectura x86, integrando las etapas fundamentales del proceso de compilación:
- Análisis léxico
- Análisis sintáctico
- Generación de código ensamblador
- Optimización básica

El proyecto tiene como propósito fortalecer la capacidad de:
- Análisis
- Diseño
- Implementación de sistemas de software complejos
- Evaluación experimental mediante comparación con herramientas de uso extendido en la industria

### Objetivos Específicos

Los estudiantes deberán:

1. **Seleccionar un lenguaje de programación** con características bien definidas y documentación formal.
   
   Debe incluir, como mínimo, las siguientes características:
   
   **Básicas:**
   - Tipos de datos básicos y definidos por el usuario
   - Variables y manejo de alcance (*scope*)
   - Funciones
   - Estructuras de control
   - Struct, arreglos y cadenas de caracteres (*strings*)
   
   **Avanzadas:**
   - Punteros, direccionamiento de memoria y manejo de memoria dinámica
   - Tipos genéricos y plantillas (*templates*)
   - Inferencia, conversión y promoción automática de tipos
   - Arreglos multidimensionales y funciones lambda

2. Implementar un compilador completo que incluya las fases de:
   - Análisis léxico
   - Análisis sintáctico
   - Análisis semántico

3. Implementar un sistema de verificación de tipos y manejo de errores:
   - Errores léxicos
   - Errores sintácticos
   - Errores semánticos

4. Generar código ensamblador para arquitectura x86.

5. Aplicar técnicas básicas de optimización sobre el código generado.

6. Desarrollar un conjunto de *benchmarks* para evaluar el rendimiento del compilador.

7. Realizar una comparación experimental con compiladores de uso extendido como:
   - Rustc
   - Go Compiler

8. Analizar y documentar los resultados obtenidos mediante:
   - Tablas
   - Gráficos
   - Discusión técnica

9. Presentar y defender el proyecto mediante una exposición técnica.

### Máximo Logro (Bonus: hasta 3 puntos)

El máximo logro consistirá en la implementación de una **aplicación funcional del compilador** que permita demostrar, de manera integrada, las distintas fases del proceso de compilación y las principales características del lenguaje implementado.

La aplicación deberá incluir, como mínimo, los siguientes componentes:

- **Editor de código** para el lenguaje diseñado
- **Visualización del AST** (*Abstract Syntax Tree*)
- **Generación de código ensamblador x86**
- **Ejecución o simulación** del programa compilado
- **Visualización de resultados de ejecución**

### Rúbrica de Calificación

| Criterio | Excelente | Bueno | Regular | Deficiente | Pts |
|----------|-----------|-------|---------|-----------|-----|
| **Diseño y Lexer** | Lenguaje consistente y lexer robusto con manejo adecuado de errores. | Diseño adecuado y lexer funcional con pocos errores. | Diseño limitado o lexer con fallos ocasionales. | Diseño incompleto o lexer incorrecto. | 2 |
| **Sintaxis y AST** | Parser correcto, AST bien estructurado y tabla de símbolos organizada. | Implementación funcional con leves limitaciones. | Implementación parcial o poco organizada. | Implementación incorrecta o incompleta. | 2 |
| **Análisis Semántico** | Verificación sólida de tipos y errores. | Buen manejo semántico. | Validación parcial. | Escasa validación. | 1 |
| **Generación x86** | Código eficiente, correcto y optimizado. | Código funcional con leves limitaciones. | Código poco eficiente. | Código incorrecto o incompleto. | 3 |
| **Características Avanzadas** | Implementa correctamente la mayoría de funcionalidades avanzadas. | Implementa varias funcionalidades relevantes. | Implementación parcial. | Muy pocas funcionalidades implementadas. | 2 |
| **Optimización** | Implementa optimizaciones relevantes y demuestra mejoras medibles. | Implementa algunas optimizaciones funcionales. | Optimización mínima. | No implementa optimizaciones. | 3 |
| **Comparación Comercial** | Benchmark riguroso y análisis sólido frente a compiladores comerciales. | Comparación adecuada con métricas parciales. | Comparación superficial. | No realiza comparación. | 2 |
| **Reporte Técnico** | Documentación técnica completa, clara y bien estructurada. | Documentación adecuada con algunos vacíos menores. | Documentación parcial o poco clara. | Documentación insuficiente. | 2 |
| **Exposición** | Presentación clara, dominio técnico y demostración sólida del compilador. | Buena presentación y explicación adecuada del funcionamiento. | Presentación limitada o con poco dominio del proyecto. | Escaso dominio o incapacidad de comunicar el trabajo. | 3 |

---
