.intel_syntax noprefix
.text
.global fib
fib:
  push rbp
  mov rbp, rsp
  sub rsp, 32
  mov [rbp-8], rcx
  mov rax, [rbp-8]
  push rax
  mov rax, 2
  mov rcx, rax
  pop rax
  cmp eax, ecx
  setl al
  movzx eax, al
  cmp eax, 0
  je .Lelse1
  mov rax, [rbp-8]
  leave
  ret
  jmp .Lend1
.Lelse1:
.Lend1:
  mov rax, [rbp-8]
  push rax
  mov rax, 1
  mov rcx, rax
  pop rax
  sub rax, rcx
  push rax
  pop rcx
  call fib
  push rax
  mov rax, [rbp-8]
  push rax
  mov rax, 2
  mov rcx, rax
  pop rax
  sub rax, rcx
  push rax
  pop rcx
  call fib
  mov rcx, rax
  pop rax
  add rax, rcx
  leave
  ret
  mov eax, 0
  leave
  ret
.global main
main:
  push rbp
  mov rbp, rsp
  sub rsp, 16
  mov rax, 10
  push rax
  pop rcx
  call fib
  leave
  ret
  mov eax, 0
  leave
  ret
.global DaYin
DaYin:
  push rbp
  mov rbp, rsp
  sub rsp, 32
  call puts
  leave
  ret
.data
.bss
