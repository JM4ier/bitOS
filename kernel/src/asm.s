.intel_syntax noprefix
.global jump
jump:
    jmp rdi

.global switch_context
switch_context:
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    pushfq
    mov QWORD PTR [rdi], rsp
    mov rsp, rsi 
    mov cr3, rdx
    popfq
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    ret

