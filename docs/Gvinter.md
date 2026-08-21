# Gvinter 网络驱动

Gvinter 是 Gvtcier 内置的网络驱动，提供以太网与 TCP/IP 协议支持。

## 技术线：协议栈分层

Gvinter 采用自底向上的分层设计，每层只处理自己职责内的字段：

```
应用层      DNS / HTTP / DHCP / 套接字接口(taojie_* / raw_*)
传输层      TCP(连接表/拥塞控制/重传)  UDP(通用收发)
网络层      IPv4(校验和验证)  IPv6(NDP 邻居发现)
链路层      ARP(缓存表)  以太网帧封装
硬件层      RTL8139(环形接收缓冲 / 发送缓冲)  轮询收包
```

- **收包路径**：`poll()` 读网卡环形缓冲 → 按以太网类型分发（0x0806 ARP / 0x0800 IPv4 / 0x86DD IPv6）→ 逐层解析
- **发包路径**：上层构造载荷 → `ipv4_send` 查 ARP 缓存决定目标 MAC → `eth_send` 封装帧 → `send_frame` 写网卡
- **接收侧校验**：IPv4 头校验和验证，无效帧丢弃；TCP 段头长度校验（hlen 越界防御）

## 功能

- 以太网收发
- ARP 地址解析与缓存（8 条，发送前查缓存，应答自动学习）
- IPv4 与 IPv6（NDP：NS 应答 NA、RA 网关学习）
- ICMP 与 ICMPv6 应答（ping）
- TCP：连接表 8 路复用、拥塞控制（慢启动/拥塞避免/超时重传）、FIN 状态机
- UDP 通用收发接口（udp_bind/send_to/recv_from）
- DHCP 客户端（DISCOVER/OFFER/REQUEST/ACK，动态获取 IP）
- DNS 查询、HTTP 客户端 + 服务器
- 原始套接字（raw_open/send/recv，帧级收发）
- WiFi 编程接口（为无线驱动预留）

## 网卡

驱动适配 Realtek RTL8139 网卡，支持 QEMU 与真实硬件。

- 初始化：PCI 枚举定位设备 → 读取 MAC → 配置接收/发送缓冲 → 置接收使能
- 接收：环形缓冲（8192+16 字节），`poll()` 轮询 CBR/CAPR 指针增量读取
- 发送：单发送缓冲，`send_frame` 拷贝帧数据后触发发送

## 验证

网络协议栈逻辑由 `GvtcierXuNiJi`（最小虚拟机）仿真验证：初始化序列、帧构造、校验和与协议字段均通过规范校验。详情见 `GvtcierXuNiJi` 源码。
