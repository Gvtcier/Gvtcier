# ABI 文档

ABI（应用二进制接口）是 Gvtcier 引导层、内核与开发者之间的共享契约，位于 `Kernel/abi`。

## 技术线：ABI 层在系统中的位置

ABI 是编译期与运行期的双重契约：

- **编译期**：`KERNEL_VIRT`、`BootInfo`、`MemoryRegion` 等常量与结构体被内核、引导层、用户程序共同引用,保证三端对地址空间与内存地图的理解一致
- **运行期**：`Sys` 模块封装 1-40 号系统调用,作为用户态访问内核服务的唯一入口;`Gvtcier2D` 封装图形基础设施

```
引导层(填充 BootInfo)──▶ 内核入口(读取 BootInfo)──▶ 用户程序
                              │
                              └── Sys 封装 ──syscall──▶ 内核服务
```

## 目录结构

```
Kernel/abi/
├── src/
│   ├── lib.rs          ABI 入口（常量 + 引导结构体）
│   ├── Gvtcier2D.rs     图形基础设施（g2d_* 图形 API）
│   └── Sys.rs          内核功能 API（syscall 封装）
└── data/               汉字字库（font16.bin + unicode.bin）
```

## 常量

- `KERNEL_VIRT`：内核高半区虚拟地址基址（`0xFFFF800000000000`）

> **为什么高半区？** x86-64 虚拟地址空间被分为低半区（用户态，0x0000…）与高半区（内核态，0xFFFF…）。内核代码链接于 KERNEL_VIRT 起始的高半区,用户态程序无法访问该地址范围,天然隔离内核与用户空间;同时高半区为所有进程共享,用户进程页表只需映射一次内核页表即可访问内核（经系统调用）。

## 引导结构体

| 结构体 | 字段 | 说明 |
|---|---|---|
| `BootInfo` | mem_map_addr / mem_map_len | 内存映射地址与长度 |
| | fb_addr / fb_width / fb_height / fb_stride / fb_pixel_format | 帧缓冲信息 |
| `MemoryRegion` | start / len / kind | 内存区域（kind：0=可用、1=保留） |

> **BootInfo 从哪来？** 引导层（BIOS stage2 / UEFI bootloader）在进入内核前,通过 e820 中断（BIOS）或固件协议（UEFI）探测物理内存,将结果整理为 `MemoryRegion` 数组,连同帧缓冲信息一起以 `BootInfo` 结构传给内核入口。内核据此初始化 Buddy 分配器与帧缓冲,不重复探测。

## 使用教程

**第一步**：在 `Cargo.toml` 添加依赖：

```toml
[dependencies]
gvtcier_abi = { path = "Kernel/abi" }
```

**第二步**：引入模块：

```rust
use gvtcier_abi::BootInfo;
use gvtcier_abi::MemoryRegion;
use gvtcier_abi::Gvtcier2D; // 图形 API（g2d_*）
use gvtcier_abi::Sys;      // 内核功能 API
```

**第三步**：读取引导信息（内核入口处）：

```rust
let boot = &*(info_addr as *const BootInfo);
let fb = boot.fb_addr as *mut u8;
// fb_width / fb_height / fb_stride 用于绘制画布
```

## 下一步

图形绘制见 `Gvtcier2D 使用教程`，内核功能调用见 `API 文档`。
