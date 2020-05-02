.intel_syntax noprefix

.global syscall_handler
syscall_handler:
    push rbp
    push rbx
    push r12
    push r13 
    push r14
    push r15

    mov r15, rsp
    mov rsp, 0x7000000fffff # KERNEL_SYSCALL_STACK_TOP
    push r15
    call __syscall
    pop r15
    mov rsp, r15
    
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    iretq

