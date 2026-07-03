.section .rodata
.LC_print_i64:
    .string "%ld\n"
.LC_print_string:
    .string "%s\n"
.text

partition:
    pushq	%rbp
    movq	%rsp, %rbp
    subq	$32, %rsp
    movq	%rdi, -8(%rbp)
    movq	%rsi, -16(%rbp)
    movq	%rdx, -24(%rbp)
.Lpartition_entry:
    movq	-8(%rbp), %rax
    movq	-24(%rbp), %r10
    imulq	$8, %r10
    addq	%r10, %rax
    movq	0(%rax), %r11
    movq	%r11, -32(%rbp)
    movq	-32(%rbp), %rax
.Lpartition_end:
    leave
    ret

quicksort_range:
    pushq	%rbp
    movq	%rsp, %rbp
    subq	$64, %rsp
    movq	%rdi, -8(%rbp)
    movq	%rsi, -16(%rbp)
    movq	%rdx, -24(%rbp)
.Lquicksort_range_entry:
    movq	-16(%rbp), %rax
    movq	-24(%rbp), %r10
    cmpq	%r10, %rax
    setl	%al
    movzbq	%al, %rax
    movq	%rax, -32(%rbp)
    cmpq	$0, %rax
    jne	.Lquicksort_range_if_then_0
    jmp	.Lquicksort_range_if_else_1
.Lquicksort_range_if_then_0:
    movq	-8(%rbp), %rdi
    movq	-16(%rbp), %rsi
    movq	-24(%rbp), %rdx
    call	partition
    movq	%rax, -40(%rbp)
    movq	%rax, -56(%rbp)
    jmp	.Lquicksort_range_if_merge_2
.Lquicksort_range_if_else_1:
    movq	$0, %rax
    movq	%rax, -48(%rbp)
    movq	%rax, -56(%rbp)
.Lquicksort_range_if_merge_2:
    movq	-56(%rbp), %rax
.Lquicksort_range_end:
    leave
    ret

.globl main
main:
    pushq	%rbp
    movq	%rsp, %rbp
    subq	$32, %rsp
.Lmain_entry:
    movq	$64, %rdi
    call	malloc
    movq	%rax, -8(%rbp)
    movq	$0, %rax
    movq	%rax, -16(%rbp)
    movq	$5, %rax
    movq	%rax, -24(%rbp)
    movq	-8(%rbp), %rdi
    movq	-16(%rbp), %rsi
    movq	-24(%rbp), %rdx
    call	quicksort_range
    movq	%rax, -32(%rbp)
.Lmain_end:
    leave
    ret
.section .note.GNU-stack,"",@progbits
