.arch armv8-a
.section __TEXT,__cstring,cstring_literals
.LC_print_i64:
.asciz "%ld\n"
.text
.balign 4
.globl _main
_main:
    stp x29, x30, [sp, #-16]!
    ldr x0, =.LC_print_i64
    mov x1, #42
    bl _printf
    mov w0, #0
    ldp x29, x30, [sp], #16
    ret
