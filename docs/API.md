# API 文档（内核功能 API——Sys 模块）

`Sys` 模块封装了 Gvtcier 内核的系统调用，供开发者直接调用。位于 `Kernel/abi/src/Sys.rs`。

## 引入

```rust
use gvtcier_abi::Sys;
```

## 函数一览

### 输出与调度

| 函数 | 对应 syscall | 说明 |
|---|---|---|
| `Sys::book(ptr, len)` | 1 | 写输出（串口/控制台） |
| `Sys::yield_now()` | 2 | 让出 CPU |
| `Sys::rest(ticks)` | 24 | 休眠指定 tick 数 |

### IPC 与权能

| 函数 | 对应 syscall | 说明 |
|---|---|---|
| `Sys::talk_create()` | 3 | 创建端点（返回端点编号） |
| `Sys::shout(cap, ptr, len)` | 4 | 发送消息 |
| `Sys::listen(cap, ptr, nonblock)` | 5 | 接收消息（nonblock=1 非阻塞） |

### 存储

| 函数 | 对应 syscall | 说明 |
|---|---|---|
| `Sys::plate_read(lba, count, buf)` | 9 | 读磁盘扇区 |

### 内存

| 函数 | 对应 syscall | 说明 |
|---|---|---|
| `Sys::bind(vaddr, phys, pages, flags)` | 22 | 映射地址空间 |
| `Sys::unbind(vaddr, pages)` | 23 | 解除映射 |

## 使用教程

### 写输出

```rust
let msg = b"hello harLin";
Sys::book(msg.as_ptr() as usize, msg.len());
```

### 创建端点并通信

```rust
let ep = Sys::talk_create();
// 发送
Sys::shout(ep, data.as_ptr() as usize, data.len());
// 接收（非阻塞）
let n = Sys::listen(ep, buf.as_ptr() as usize, 1);
```

### 休眠

```rust
Sys::rest(100); // 休眠 100 tick
```

## 下一步

图形绘制见 `Gvtcier2D 使用教程`，ABI 契约见 `ABI 文档`。
