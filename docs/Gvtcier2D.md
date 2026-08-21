# Gvtcier2D 使用教程

Gvtcier2D 是 Gvtcier 的图形基础设施，位于 `Kernel/abi/src/Gvtcier2D.rs`，提供画布绘制 API（前缀 `g2d_`）。

## 技术线：从绘制到显示

Gvtcier2D 采用**软件帧缓冲 + 画布合成**模型,不依赖 GPU:

```
绘制 API(g2d_paint/box/stroke/glyph/curve…)
  → 写入画布缓冲(内存中的像素数组)
  → g2d_canvas_create/map 注册画布
  → g2d_compose 将画布合成到屏幕帧缓冲(x, y 偏移)
  → g2d_flush 刷新显示(复制到硬件帧缓冲)
```

- **像素格式**:ARGB8888,每个像素 4 字节(alpha、red、green、blue),颜色字面量如 `0x3498DBFF`
- **画布模型**:画布是独立于屏幕的内存缓冲,可在离屏绘制后一次性合成,支持分层与局部刷新
- **坐标体系**:所有 API 以左上角为原点,x 向右、y 向下,与屏幕帧缓冲一致

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

### 合成流程

```
g2d_canvas_create(width, height, buf)  声明画布(大小 + 指向用户提供的像素缓冲)
g2d_canvas_map(canvas)                 注册画布到内核画布表(上限 16)
g2d_compose(canvas, x, y)              将画布像素按偏移复制到屏幕帧缓冲
g2d_flush()                            将帧缓冲整体刷新到显示硬件
```

- 画布创建时只登记元数据,实际像素存于用户缓冲,绘制 API 直接写该缓冲
- `compose` 做像素复制(可含裁剪),`flush` 触发硬件级刷新

## 汉字字库

Gvtcier2D 内置 GB2312 全部 6763 个常用汉字的 16x16 点阵字库（`Kernel/abi/data/font16.bin` + `unicode.bin`），`g2d_script` 直接支持 UTF-8 汉字文本。

### 字库技术线

```
UTF-8 字节 → 解码为 Unicode 码点 → 映射到 GB2312 区位
  → 查 font16.bin(16x16 点阵,每字 32 字节)
  → 按位展开为像素,写入画布
```

- 英文字符走 8x8 内置点阵,汉字走 16x16 字库
- `unicode.bin` 提供 Unicode→GB2312 的映射表,支持任意 UTF-8 输入文本

## 下一步

内核功能调用见 `API 文档`，ABI 契约见 `ABI 文档`。
