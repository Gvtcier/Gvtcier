# Gvinter 网络驱动

Gvinter 是 Gvtcier 内置的网络驱动,提供以太网与 TCP/IP 协议支持。

## 功能

- 以太网收发
- ARP 地址解析
- IPv4 与 IPv6 基础
- ICMP 与 ICMPv6 应答
- TCP 连接与数据回显
- WiFi 编程接口(为无线驱动预留)

## 网卡

驱动适配 Realtek RTL8139 网卡,支持 QEMU 与真实硬件。

## 验证

网络协议栈逻辑由 `GvtcierXuNiJi`(最小虚拟机)仿真验证:初始化序列、帧构造、校验和与协议字段均通过规范校验。详情见 `GvtcierXuNiJi` 源码。
