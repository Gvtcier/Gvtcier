# Gvtcier2D 使用教程

Gvtcier2D 是 Gvtcier 的图形基础设施，位于 `Kernel/abi/src/Gvtcier2D.rs`，提供画布绘制 API（前缀 `g2d_`）。

## 引入

```rust
use gvtcier_abi::Gvtcier2D;
```

## 绘制原语

### 敷（填充/清屏）

```rust
Gvtcier2D::g2d_paint(buf, width, height, 0x101820FF); // 全屏填充颜色
```

### 框（矩形）

```rust
Gvtcier2D::g2d_box(buf, width, x, y, w, h, 0x3498DBFF); // 实心矩形
```

### 线（直线）

```rust
Gvtcier2D::g2d_stroke(buf, width, x1, y1, x2, y2, 0xE8E8E8FF); // 画线
```

### 字（字符）/ 文（文本）

```rust
Gvtcier2D::g2d_glyph(buf, width, x, y, b'A', 0xFFFFFF00); // 单个字符
Gvtcier2D::g2d_script(buf, width, x, y, "你好".as_bytes(), 0xE8E8E8FF); // UTF-8 文本（含汉字）
```

### 贴（贴图/区域复制）

```rust
Gvtcier2D::g2d_paste(buf, width, src, x, y, w, h); // 复制区域
```

### 圆 / 椭圆

```rust
Gvtcier2D::g2d_disc(buf, width, cx, cy, r, 0xE74C3CFF); // 实心圆
Gvtcier2D::g2d_ellipse(buf, width, cx, cy, rx, ry, 0x9B59B6FF); // 实心椭圆
```

### 渐变 / 圆角框

```rust
Gvtcier2D::g2d_gradient(buf, width, height, x, y, w, h, 0xFF0000, 0x0000FF); // 垂直渐变
Gvtcier2D::g2d_round_box(buf, width, x, y, w, h, 8, 0x182830FF); // 圆角矩形
```

### 曲线（贝塞尔）

```rust
let pts = [(100, 300), (400, 100), (700, 500), (1000, 300)];
Gvtcier2D::g2d_curve(buf, width, &pts, 0x4FC3F7FF, 64); // n 次贝塞尔曲线（64 段）
```

## 画布合成（显示到屏幕）

```rust
let canvas = Gvtcier2D::g2d_canvas_create(width, height, buf as u64);
Gvtcier2D::g2d_canvas_map(canvas);
Gvtcier2D::g2d_compose(canvas, 0, 0); // 合成到屏幕 (x, y)
Gvtcier2D::g2d_flush();               // 刷新显示
```

## 汉字字库

Gvtcier2D 内置 GB2312 全部 6763 个常用汉字的 16x16 点阵字库（`Kernel/abi/data/font16.bin` + `unicode.bin`），`g2d_script` 直接支持 UTF-8 汉字文本。

## 下一步

内核功能调用见 `API 文档`，ABI 契约见 `ABI 文档`。
