@echo off
rem Gvtcier v0.2
cd /d %~dp0
if not exist ovmf_vars.fd fsutil file createnew ovmf_vars.fd 262144 >nul
if not exist out\log mkdir out\log
"D:\Code\Local_tool_library\qemu\qemu-system-x86_64.exe" -machine q35 -drive if=pflash,format=raw,unit=0,file=D:\Code\Local_tool_library\qemu\share\edk2-x86_64-code.fd,readonly=on -drive if=pflash,format=raw,unit=1,file=ovmf_vars.fd -cdrom out\gvtcier.iso -serial file:out\log\serial.log -display gtk
