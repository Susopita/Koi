.arch armv8-a
.section __TEXT,__const
.balign 8
.LC_print_i64:
.asciz "%ld\n"
.text
.balign 4
.globl _main
_main:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    adrp x0, .LC_print_i64@PAGE
    add x0, x0, .LC_print_i64@PAGEOFF
    mov x1, #42
    str x1, [sp, #-16]!
    adrp x2, .LC_print_i64@PAGE
    add x2, x2, .LC_print_i64@PAGEOFF
    bl _printf
    ldr x1, [sp], #16
    mov w0, #0
    ldp x29, x30, [sp], #16
    ret
