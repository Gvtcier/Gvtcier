# GVPIR —— Gvtcier 专有接口参考

Gvtcier Proprietary Interface Reference，简称 GVPIR，是 Gvtcier 操作系统的专有应用二进制接口参考，定义了系统调用编号与语义、调用约定、返回值约定、运行时环境与进程模型。

GVPIR 是对内核分发与 ABI 封装层的事实标准化，也是 Gvtcier 的自有接口规范。遵循 GVPIR 开发的程序，可在 UEFI 与 BIOS 两种引导模式上一致运行。

GVPIR 是 Gvtcier 的专有接口，接口形态完全自定义：进程模型采用裂变，IPC 基于端点消息与权能，图形与文件为自制体系，不依赖任何外部 ABI 标准。

## 1. 系统调用编号

系统调用通过 syscall 指令进入内核，编号放在 rax。参数依次放入 rdi、rsi、rdx、r10、r9、r8，返回值写回 SYS_RET，由 ABI 封装读取。

| 编号 | 名称 | 说明 | ABI 封装 |
|---|---|---|---|
| 1 | book | 写串口，打印字节串 | Sys::book |
| 2 | yield | 让出，触发任务调度 | Sys::yield_now |
| 3 | talk_create | 创建 IPC 端点并分配权能 | Sys::talk_create |
| 4 | shout | 经端点发送消息 | Sys::shout |
| 5 | listen | 从端点接收消息，支持阻塞 | Sys::listen |
| 6 | nod | 返回固定信标 | — |
| 7 | draw_char | 帧缓冲写一个字符 | — |
| 8 | clear | 清屏 | Fb::clear |
| 9 | plate_read | 读 LBA 扇区 | Sys::plate_read |
| 10 | drv_register | 注册驱动端点 | — |
| 11 | drv_lookup | 按权能查驱动 | — |
| 12 | audio_play | 播放音频 | Audio::play |
| 13-17 | 画布管理 | 画布创建、销毁、缓冲、合成 | Gfx::* |
| 22 | bind | 映射物理页 | Sys::bind |
| 23 | unbind | 解除映射 | Sys::unbind |
| 24 | rest | 延时 N 个 tick | Sys::rest |
| 25-29 | 2D 绘制 | 填充、矩形、直线、字符、文本 | Gvtcier2D::g2d_* |
| 30-32 | 滚动文件 | 打开、读取、关闭 | Sys::scroll_* |
| 33 | leaf_alloc | 分配页 | Sys::leaf_alloc |
| 34 | leaf_free | 释放页 | Sys::leaf_free |
| 35 | bezier | 贝塞尔曲线 | — |
| 36 | write | 写 LBA 扇区 | Ahci::write |
| 37 | file_write | 文件写入 | File::write |
| 38 | create_task | 创建任务 | Task::create_task |
| 39 | getuptime | 返回系统运行 tick 数 | Sys::pic |

编号 18-21 与 40 以上保留。19 是进程派生的占位，后续定义。

## 2. 调用约定

- 指令：syscall，返回地址在 rcx，rflags 在 r11。
- 参数寄存器：rdi、rsi、rdx、r10、r9、r8。
- 系统调用号：rax。
- 返回值：写回 SYS_RET，由 ABI 封装读取。
- 构建：x86_64-unknown-none 与 x86_64-unknown-uefi 裸机目标。

## 3. 返回值约定

| 值 | 含义 |
|---|---|
| 0 | 成功；对返回句柄或数的调用，见具体语义 |
| 0xFFFFFFFFFFFFFFFF | 分配失败 |
| 2 | 权能与权限错误 |
| >= 1 | 句柄、字节数或端点 id |

GVPIR 采用返回码，不采用异常，调用方自行检查。

## 4. 运行时环境

- 模型：任务为可调度单元，用户程序作为任务加载；进程采用裂变模型。见 docs/FISSION.md。
- IPC：端点与权能，发送与接收依赖权能。
- 文件：GvFAT 文件系统，提供滚动窗口文件接口。
- 内存：页粒度分配与释放，虚拟地址映射。
- 图形：画布与 2D 绘制原语。

## 5. 接口设计原则

Gvtcier 的接口体系全部为自制形态。

| 概念 | Gvtcier 的实现 |
|---|---|
| 进程派生 | 裂变模型 |
| 进程间通信 | 端点消息与权能 |
| 时间 | getuptime |
| 内存分配 | 页分配与释放、地址映射 |
| 设备 IO | 驱动注册与端点 |
| 文件系统 | GvFAT |

## 6. 扩展

GVPIR 编号在 ABI 封装与内核分发中成对维护。新增系统调用时，内核增加对应分支，ABI 增加封装函数，返回值写回白名单须与有返回值的编号同步。
