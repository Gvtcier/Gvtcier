# GvtcierC

GvtcierC 是自制编程语言，位于 GvtcierC\ 目录。包含编译器 gvtcierc 与汇编器/链接器 gvtcierk，不依赖 gcc。

## 关键字

| 关键字 | 含义 |
|--------|------|
| ZhengShu | 整数类型 |
| FuDian | 单精度浮点类型 |
| ShuangJing | 双精度浮点类型 |
| ZiFu | 字符类型（可用于字符数组） |
| RuGuo | 条件分支（if） |
| FouZe | 否则（else） |
| Dang | 循环（while） |
| XunHuan | 循环（for） |
| JiXu | 继续循环（continue） |
| TiaoChu | 跳出循环（break） |
| FanHui | 返回值（return） |
| DaYin | 内置打印函数（输出字符串或整数） |

## 编译器命令

### gvtcierc

编译一个 .gc 源文件：

```
gvtcierc <file.gc>
```

流程：gvtcierc 将 .gc 编译为 .s（x86-64 汇编），随后调用同目录的 gvtcierk 汇编并链接为 .exe（Windows 可执行文件），全程不依赖 gcc。

产物：

- `<file>.s`：汇编文件
- `<file>.exe`：可执行文件

### gvtcierk

汇编并链接一个 .s 文件：

```
gvtcierk <file.s>
```

生成 `<file>.exe`，包含最小运行时（打印与文件读写函数）。

## 示例

demo.gc：

```
ZhengShu add(ZhengShu a, ZhengShu b) {
    FanHui a + b;
}
ZhengShu main() {
    DaYin("sum=");
    DaYin(add(3, 4));
    FanHui 0;
}
```

编译并运行：

```
gvtcierc demo.gc
demo.exe
```

输出：

```
sum=7
```
