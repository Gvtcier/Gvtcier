# Gvt 版本控制

Gvt 是版本控制工具，位于 Gvt\ 目录，源码为 Gvt\src\gvt.c。

## 命令

- chushi 初始化仓库，创建 .gvt 目录
- jia <文件或目录> 添加文件到索引，目录会递归添加
- tijiao -m <消息> 提交变更
- rizhi [-n <数量>] 查看提交历史
- bangzhu 查看帮助

## 排除文件

PaiChu.gvt 列出不需要提交的文件或目录，每行一个名称。目录按名称排除整个目录。

## 使用

初始化后添加并提交：

```
gvt chushi
gvt jia .
gvt tijiao -m "更新说明"
gvt rizhi
```
