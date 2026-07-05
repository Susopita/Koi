.arch armv8-a
.section __TEXT,__const
.balign 8
.LC_print_i64:
.asciz "%ld\n"
.LC_print_string:
.asciz "%s\n"
.LC_print_f64:
.asciz "%f\n"

.text
.balign 4
.globl fib
fib:
	stp x29, x30, [sp, #-16]!
	stp x19, x20, [sp, #-16]!
	stp x21, x22, [sp, #-16]!
	stp x23, x24, [sp, #-16]!
	stp x26, x27, [sp, #-16]!
	mov x29, sp
	mov x19, x0
	movz x20, #2
	cmp x19, x20
	cset x21, lt
	cmp x21, #0
	b.ne .Lfib_if_then_0
	b .Lfib_if_else_1
.Lfib_if_then_0:
	mov x26, x19
	b .Lfib_if_merge_2
.Lfib_if_else_1:
	movz x24, #1
	sub x23, x19, x24
	mov x0, x23
	bl fib
	mov x27, x0
	movz x20, #2
	sub x21, x19, x20
	mov x0, x21
	bl fib
	mov x22, x0
	add x19, x27, x22
	mov x26, x19
	b .Lfib_if_merge_2
.Lfib_if_merge_2:
	mov x0, x26
	b .Lfib_end
.Lfib_end:
	ldp x26, x27, [sp], #16
	ldp x23, x24, [sp], #16
	ldp x21, x22, [sp], #16
	ldp x19, x20, [sp], #16
	ldp x29, x30, [sp], #16
	ret
.balign 4
.globl main
.globl _main
_main:
main:
	stp x29, x30, [sp, #-16]!
	stp x19, x20, [sp, #-16]!
	str x21, [sp, #-16]!
	mov x29, sp
	movz x21, #32
	mov x0, x21
	bl fib
	mov x20, x0
	str x20, [sp, #-16]!
	adrp x0, .LC_print_i64@PAGE
	add x0, x0, .LC_print_i64@PAGEOFF
	bl _printf
	add sp, sp, #16
	mov x19, x0
	mov x0, x19
	b .Lmain_end
.Lmain_end:
	ldr x21, [sp], #16
	ldp x19, x20, [sp], #16
	ldp x29, x30, [sp], #16
	ret
