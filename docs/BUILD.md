# 编译流程文档

本教程讲解 Gvtcier 的编译、打包与运行流程。

## 引导模式

Gvtcier 支持 BIOS 与 UEFI 两种引导模式，打包为两个独立镜像：

- `out/Gvtcier_x86_64.iso`：默认从 UEFI 引导
- `out/Gvtcier_x86_32.iso`：默认从 BIOS 兼容引导

BIOS 模式经 32 位保护模式兼容层进入 64 位内核；UEFI 模式由固件直接引导。两种模式共用同一套内核，仅引导层不同。

产物命名规范（未来架构扩展）：

| 架构 | 格式 | 产物 |
|------|------|------|
| x86_64 UEFI | ISO | Gvtcier_x86_64.iso |
| x86_32 BIOS | ISO | Gvtcier_x86_32.iso |
| ARM64 | RAW | Gvtcier_ARM64.raw |
| ARM32 | RAW | Gvtcier_ARM32.raw |
| RISC-V 64 | RAW | Gvtcier_RISCV64.raw |
| RISC-V 32 | RAW | Gvtcier_RISCV32.raw |

嵌入式目标输出为裸镜像（image）。

## 环境要求

- Rust 工具链
- 目标 x86_64-unknown-none 与 x86_64-unknown-uefi
- NASM

## 编译各 crate

所有 crate 的构建产物统一输出到仓库根目录的 `out/`。宿主构建在 `out/release/`，交叉构建在 `out/<目标三元组>/release/`。

### 1. ABI 基础设施

```bash
cd Kernel/abi
cargo build --release
```

### 2. 内核

```bash
cd Kernel/kernel
cargo build --release
```

注意：内核 crate 设定了构建目标，须在内核目录内构建。

### 3. 引导层

```bash
cd Kernel/bootloader
cargo build --release
```

### 4. 驱动

```bash
cd Kernel/Drive/fat
cargo build --release
cd Kernel/Drive/gvfat
cargo build --release
cd Kernel/Drive/kbd
cargo build --release
```

### 5. BIOS 引导

仅修改引导源码时需要重新汇编。

```bash
cd BIOS
nasm -f bin boot.asm -o boot.bin
nasm -f bin stage2.asm -o stage2.bin
```

## 打包 ISO

gcx 打包与 ISO 打包分两步：

```bash
# 1) ELF 转 gcx
out\release\gvtcier-gcx.exe out/x86_64-unknown-none/release/gvtcier-kernel kernel.gcx

# 2) 打包 UEFI 默认镜像
out\release\gvtcier-iso.exe out/x86_64-unknown-uefi/release/gvtcier-bootloader.efi kernel.gcx out/Gvtcier_x86_64.iso uefi

# 3) 打包 BIOS 兼容默认镜像
out\release\gvtcier-iso.exe out/x86_64-unknown-uefi/release/gvtcier-bootloader.efi kernel.gcx out/Gvtcier_x86_32.iso bios
```

`gvtcier-iso` 第 5 个参数指定默认引导方式：`uefi` 或 `bios`。

## 运行

### UEFI 模式

```bash
qemu-system-x86_64 -machine q35 \
  -drive if=pflash,format=raw,unit=0,file=<OVMF_CODE.fd>,readonly=on \
  -drive if=pflash,format=raw,unit=1,file=ovmf_vars.fd \
  -cdrom out/Gvtcier_x86_64.iso -display gtk
```

### BIOS 模式

```bash
qemu-system-x86_64 -machine q35 \
  -cdrom out/Gvtcier_x86_32.iso -boot order=d -display gtk
```

### 挂载硬盘

硬盘挂到端口 0，光驱挂到端口 1：

```bash
qemu-system-x86_64 -machine q35 \
  -drive file=out/Gvtcier_x86_64.iso,format=raw,if=none,id=cd0,media=cdrom \
  -device ide-cd,drive=cd0,bus=ide.1 \
  -drive file=disk.img,format=raw,if=none,id=hd0 \
  -device ide-hd,drive=hd0,bus=ide.0 \
  -boot order=d -serial file:serial.log -display gtk
```

## 一键启动

`run.bat` 以 UEFI 模式启动 QEMU。

## GvShell

内核启动后进入串口 Shell，输入 `help` 查看命令。无图形输出时可全程用串口观察引导与内核日志。

## 下一步

新手入门见 `新手小白注意事项`，API 使用见 `API 文档`。
