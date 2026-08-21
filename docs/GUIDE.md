# 新手小白注意事项

本教程面向初次接触 Gvtcier 的新手，讲解常见注意事项。

## 环境准备

1. 安装 Rust 工具链（rustup）
2. 添加目标：
   ```bash
   rustup target add x86_64-unknown-none
   rustup target add x86_64-unknown-uefi
   ```
3. 准备 QEMU；UEFI 启动还需要 OVMF 固件（BIOS 启动不需要，直接用 SeaBIOS）

## 常见注意事项

1. **先编译 ABI**：内核/驱动依赖 `Kernel/abi`——先编译 ABI 再编译其他 crate
2. **目标平台**：内核/驱动用 `x86_64-unknown-none`（裸机）；引导层用 `x86_64-unknown-uefi`
3. **打包顺序**：先编译所有 crate——再打包 ISO（`--bin gvtcier-iso`）
4. **OVMF 变量盘（UEFI 模式）**：`ovmf_vars.fd` 不存在时 QEMU 启动失败——`run.bat` 会自动创建；BIOS 模式无需 OVMF
5. **运行环境**：q35 机型无传统 IDE 端口——磁盘读取用 SATA/AHCI
6. **GvShell**：内核启动后进入串口 Shell——输入 `help` 查看命令；`reboot` 重启、`halt` 停机
7. **图形 API**：绘制前先 `g2d_paint` 清屏；`g2d_compose` + `g2d_flush` 后才显示到屏幕
8. **汉字文本**：`g2d_script` 支持 UTF-8 汉字（内置 6763 字库）
9. **提交规范**：提交信息用中文，主题简短（如 `docs: 说明`）
10. **保持简单**：不添加未要求的功能——最小化改动

## 遇到问题

- 编译错误：先确认 ABI 已编译、目标平台正确
- 启动失败：先确认 ISO 打包是否成功；UEFI 模式再检查 OVMF 固件与变量盘（BIOS 模式用 `-cdrom gvtcier.iso -boot order=d`）
- 图形不显示：确认已调用 `g2d_compose` + `g2d_flush`

## 下一步

编译流程见 `编译流程文档`，API 使用见 `API 文档`，ABI 见 `ABI 文档`。
