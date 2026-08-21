# ABI 文档

ABI（应用二进制接口）是 Gvtcier 引导层、内核与开发者之间的共享契约，位于 `Kernel/abi`。

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

## 引导结构体

| 结构体 | 字段 | 说明 |
|---|---|---|
| `BootInfo` | mem_map_addr / mem_map_len | 内存映射地址与长度 |
| | fb_addr / fb_width / fb_height / fb_stride / fb_pixel_format | 帧缓冲信息 |
| `MemoryRegion` | start / len / kind | 内存区域（kind：0=可用、1=保留） |

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
