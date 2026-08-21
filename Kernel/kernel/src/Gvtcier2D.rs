#[no_mangle]
static mut G2D_RET: u64 = 0;

static FONT16: &[u8] = include_bytes!("../data/font16.bin");
static UNICODE_TABLE: &[u8] = include_bytes!("../data/unicode.bin");

pub fn g2d_canvas_create(w: u64, h: u64, buf: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + G2D_RET]",
            "syscall",
            inout("rax") 13u64 => _,
            in("rdi") w,
            in("rsi") h,
            in("rdx") buf,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        G2D_RET
    }
}

pub fn g2d_canvas_destroy(id: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + G2D_RET]",
            "syscall",
            inout("rax") 14u64 => _,
            in("rdi") id,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_canvas_map(id: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + G2D_RET]",
            "syscall",
            inout("rax") 15u64 => _,
            in("rdi") id,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        G2D_RET
    }
}

pub fn g2d_compose(id: u64, x: u64, y: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + G2D_RET]",
            "syscall",
            inout("rax") 16u64 => _,
            in("rdi") id,
            in("rsi") x,
            in("rdx") y,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn sys_remove(id: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + G2D_RET]",
            "syscall",
            inout("rax") 17u64 => _,
            in("rdi") id,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_flush() {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + G2D_RET]",
            "syscall",
            inout("rax") 18u64 => _,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

unsafe fn px(buf: *mut u8, w: usize, h: usize, x: usize, y: usize, color: u32) {
    if x >= w || y >= h {
        return;
    }
    let off = (y * w + x) * 4;
    *buf.add(off) = (color >> 24) as u8;
    *buf.add(off + 1) = (color >> 16) as u8;
    *buf.add(off + 2) = (color >> 8) as u8;
    *buf.add(off + 3) = color as u8;
}

pub fn g2d_fill(buf: *mut u8, w: usize, h: usize, color: u32) {
    unsafe {
        for y in 0..h {
            for x in 0..w {
                px(buf, w, h, x, y, color);
            }
        }
    }
}

pub fn g2d_rect(
    buf: *mut u8,
    w: usize,
    x: usize,
    y: usize,
    rw: usize,
    rh: usize,
    color: u32,
) {
    unsafe {
        for j in 0..rh {
            for i in 0..rw {
                px(buf, w, w, x + i, y + j, color);
            }
        }
    }
}

pub fn g2d_line(
    buf: *mut u8,
    w: usize,
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    color: u32,
) {
    let mut x = x1 as i32;
    let mut y = y1 as i32;
    let dx = (x2 as i32 - x1 as i32).abs();
    let dy = -(y2 as i32 - y1 as i32).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        unsafe { px(buf, w, w, x as usize, y as usize, color) }
        if x == x2 as i32 && y == y2 as i32 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

pub fn g2d_disc(
    buf: *mut u8,
    w: usize,
    cx: usize,
    cy: usize,
    r: usize,
    color: u32,
) {
    unsafe {
        for dy in 0..r * 2 {
            for dx in 0..r * 2 {
                let dx2 = dx as i64 - r as i64;
                let dy2 = dy as i64 - r as i64;
                if dx2 * dx2 + dy2 * dy2 <= (r as i64) * (r as i64) {
                    px(buf, w, w, cx + dx - r, cy + dy - r, color);
                }
            }
        }
    }
}

pub fn g2d_ellipse(
    buf: *mut u8,
    w: usize,
    cx: usize,
    cy: usize,
    rx: usize,
    ry: usize,
    color: u32,
) {
    unsafe {
        for dy in 0..ry * 2 {
            for dx in 0..rx * 2 {
                let dx2 = dx as i64 - rx as i64;
                let dy2 = dy as i64 - ry as i64;
                if dx2 * dx2 * (ry as i64 * ry as i64) + dy2 * dy2 * (rx as i64 * rx as i64)
                    <= (rx as i64 * ry as i64) * (rx as i64 * ry as i64)
                {
                    px(buf, w, w, cx + dx - rx, cy + dy - ry, color);
                }
            }
        }
    }
}

pub fn g2d_gradient(
    buf: *mut u8,
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    rw: usize,
    rh: usize,
    c1: u32,
    c2: u32,
) {
    unsafe {
        for j in 0..rh {
            let t = if rh > 1 { j as f32 / (rh - 1) as f32 } else { 1.0 };
            let r = (c1 >> 16 & 0xFF) as f32 * (1.0 - t) + (c2 >> 16 & 0xFF) as f32 * t;
            let g = (c1 >> 8 & 0xFF) as f32 * (1.0 - t) + (c2 >> 8 & 0xFF) as f32 * t;
            let b = (c1 & 0xFF) as f32 * (1.0 - t) + (c2 & 0xFF) as f32 * t;
            let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32) | 0xFF000000;
            for i in 0..rw {
                px(buf, w, w, x + i, y + j, color);
            }
        }
    }
}

pub fn g2d_round_box(
    buf: *mut u8,
    w: usize,
    x: usize,
    y: usize,
    rw: usize,
    rh: usize,
    rad: usize,
    color: u32,
) {
    unsafe {
        for j in 0..rh {
            for i in 0..rw {
                let mut in_corner = false;
                if i < rad && j < rad {
                    in_corner = (rad - i) * (rad - i) + (rad - j) * (rad - j) > rad * rad;
                } else if i >= rw - rad && j < rad {
                    in_corner =
                        (i - (rw - rad)) * (i - (rw - rad)) + (rad - j) * (rad - j) > rad * rad;
                } else if i < rad && j >= rh - rad {
                    in_corner =
                        (rad - i) * (rad - i) + (j - (rh - rad)) * (j - (rh - rad)) > rad * rad;
                } else if i >= rw - rad && j >= rh - rad {
                    in_corner = (i - (rw - rad)) * (i - (rw - rad))
                        + (j - (rh - rad)) * (j - (rh - rad))
                        > rad * rad;
                }
                if !in_corner {
                    px(buf, w, w, x + i, y + j, color);
                }
            }
        }
    }
}

pub fn cubic_bezier(
    buf: *mut u8,
    w: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    x3: usize,
    y3: usize,
    color: u32,
    segments: usize,
) {
    if segments == 0 {
        return;
    }
    let (p0x, p0y) = (x0 as f64, y0 as f64);
    let (p1x, p1y) = (x1 as f64, y1 as f64);
    let (p2x, p2y) = (x2 as f64, y2 as f64);
    let (p3x, p3y) = (x3 as f64, y3 as f64);
    let mut prev_x = p0x;
    let mut prev_y = p0y;
    for i in 1..=segments {
        let t = i as f64 / segments as f64;
        let mt = 1.0 - t;
        let bx = mt * mt * mt * p0x
            + 3.0 * mt * mt * t * p1x
            + 3.0 * mt * t * t * p2x
            + t * t * t * p3x;
        let by = mt * mt * mt * p0y
            + 3.0 * mt * mt * t * p1y
            + 3.0 * mt * t * t * p2y
            + t * t * t * p3y;
        g2d_line(
            buf,
            w,
            prev_x as usize,
            prev_y as usize,
            bx as usize,
            by as usize,
            color,
        );
        prev_x = bx;
        prev_y = by;
    }
}

fn ipow(base: f64, e: i32) -> f64 {
    let mut r = 1.0;
    for _ in 0..e {
        r *= base;
    }
    r
}

pub fn bezier(
    buf: *mut u8,
    w: usize,
    points: &[(usize, usize)],
    color: u32,
    segments: usize,
) {
    if points.len() < 2 || segments == 0 {
        return;
    }
    let n = points.len() - 1;
    let mut prev_x = points[0].0 as f64;
    let mut prev_y = points[0].1 as f64;
    for s in 1..=segments {
        let t = s as f64 / segments as f64;
        let mt = 1.0 - t;
        let mut bx = 0.0;
        let mut by = 0.0;
        let mut c = 1.0f64;
        for i in 0..=n {
            let wgt = c * ipow(mt, (n - i) as i32) * ipow(t, i as i32);
            bx += wgt * points[i].0 as f64;
            by += wgt * points[i].1 as f64;
            c = c * (n - i) as f64 / (i + 1) as f64;
        }
        g2d_line(
            buf,
            w,
            prev_x as usize,
            prev_y as usize,
            bx as usize,
            by as usize,
            color,
        );
        prev_x = bx;
        prev_y = by;
    }
}

pub fn g2d_blit(
    dst: *mut u8,
    dw: usize,
    dx: usize,
    dy: usize,
    src: *const u8,
    sw: usize,
    _sh: usize,
    sx: usize,
    sy: usize,
    bw: usize,
    bh: usize,
) {
    unsafe {
        for j in 0..bh {
            for i in 0..bw {
                let so = ((sy + j) * sw + (sx + i)) * 4;
                let off = ((dy + j) * dw + (dx + i)) * 4;
                *dst.add(off) = *src.add(so);
                *dst.add(off + 1) = *src.add(so + 1);
                *dst.add(off + 2) = *src.add(so + 2);
                *dst.add(off + 3) = *src.add(so + 3);
            }
        }
    }
}

static FONT8X8: [[u8; 8]; 96] = [
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00],
    [0x18,0x3C,0x3C,0x18,0x18,0x00,0x18,0x00],
    [0x6C,0x6C,0x6C,0x00,0x00,0x00,0x00,0x00],
    [0x6C,0x6C,0xFE,0x6C,0xFE,0x6C,0x6C,0x00],
    [0x18,0x3E,0x60,0x3C,0x06,0x7C,0x18,0x00],
    [0x62,0x66,0x0C,0x18,0x30,0x66,0x46,0x00],
    [0x3C,0x66,0x3C,0x38,0x67,0x66,0x3F,0x00],
    [0x18,0x18,0x30,0x00,0x00,0x00,0x00,0x00],
    [0x0C,0x18,0x30,0x30,0x30,0x18,0x0C,0x00],
    [0x30,0x18,0x0C,0x0C,0x0C,0x18,0x30,0x00],
    [0x00,0x6C,0x38,0xFE,0x38,0x6C,0x00,0x00],
    [0x00,0x18,0x18,0x7E,0x18,0x18,0x00,0x00],
    [0x00,0x00,0x00,0x00,0x00,0x18,0x18,0x30],
    [0x00,0x00,0x00,0x7E,0x00,0x00,0x00,0x00],
    [0x00,0x00,0x00,0x00,0x00,0x18,0x18,0x00],
    [0x02,0x06,0x0C,0x18,0x30,0x60,0x40,0x00],
    [0x3C,0x66,0x6E,0x76,0x66,0x66,0x3C,0x00],
    [0x18,0x38,0x18,0x18,0x18,0x18,0x7E,0x00],
    [0x3C,0x66,0x06,0x0C,0x18,0x30,0x7E,0x00],
    [0x3C,0x66,0x06,0x1C,0x06,0x66,0x3C,0x00],
    [0x0C,0x1C,0x3C,0x6C,0x7E,0x0C,0x0C,0x00],
    [0x7E,0x60,0x7C,0x06,0x06,0x66,0x3C,0x00],
    [0x1C,0x30,0x60,0x7C,0x66,0x66,0x3C,0x00],
    [0x7E,0x06,0x0C,0x18,0x30,0x30,0x30,0x00],
    [0x3C,0x66,0x66,0x3C,0x66,0x66,0x3C,0x00],
    [0x3C,0x66,0x66,0x3E,0x06,0x0C,0x38,0x00],
    [0x00,0x18,0x18,0x00,0x00,0x18,0x18,0x00],
    [0x00,0x18,0x18,0x00,0x00,0x18,0x18,0x30],
    [0x0C,0x18,0x30,0x60,0x30,0x18,0x0C,0x00],
    [0x00,0x00,0x7E,0x00,0x7E,0x00,0x00,0x00],
    [0x30,0x18,0x0C,0x06,0x0C,0x18,0x30,0x00],
    [0x3C,0x66,0x06,0x0C,0x18,0x00,0x18,0x00],
    [0x3C,0x66,0x6E,0x6E,0x60,0x62,0x3C,0x00],
    [0x3C,0x66,0x66,0x7E,0x66,0x66,0x66,0x00],
    [0x7C,0x66,0x66,0x7C,0x66,0x66,0x7C,0x00],
    [0x3C,0x66,0x60,0x60,0x60,0x66,0x3C,0x00],
    [0x78,0x6C,0x66,0x66,0x66,0x6C,0x78,0x00],
    [0x7E,0x60,0x60,0x78,0x60,0x60,0x7E,0x00],
    [0x7E,0x60,0x60,0x78,0x60,0x60,0x60,0x00],
    [0x3C,0x66,0x60,0x6E,0x66,0x66,0x3C,0x00],
    [0x66,0x66,0x66,0x7E,0x66,0x66,0x66,0x00],
    [0x7E,0x18,0x18,0x18,0x18,0x18,0x7E,0x00],
    [0x3E,0x0C,0x0C,0x0C,0x0C,0x6C,0x38,0x00],
    [0x66,0x6C,0x78,0x70,0x78,0x6C,0x66,0x00],
    [0x60,0x60,0x60,0x60,0x60,0x60,0x7E,0x00],
    [0x63,0x77,0x7F,0x6B,0x63,0x63,0x63,0x00],
    [0x66,0x76,0x7E,0x7E,0x6E,0x66,0x66,0x00],
    [0x3C,0x66,0x66,0x66,0x66,0x66,0x3C,0x00],
    [0x7C,0x66,0x66,0x7C,0x60,0x60,0x60,0x00],
    [0x3C,0x66,0x66,0x66,0x6E,0x3C,0x07,0x00],
    [0x7C,0x66,0x66,0x7C,0x6C,0x66,0x66,0x00],
    [0x3C,0x66,0x60,0x3C,0x06,0x66,0x3C,0x00],
    [0x7E,0x18,0x18,0x18,0x18,0x18,0x18,0x00],
    [0x66,0x66,0x66,0x66,0x66,0x66,0x3C,0x00],
    [0x66,0x66,0x66,0x66,0x66,0x3C,0x18,0x00],
    [0x63,0x63,0x63,0x6B,0x7F,0x77,0x63,0x00],
    [0x66,0x66,0x3C,0x18,0x3C,0x66,0x66,0x00],
    [0x66,0x66,0x66,0x3C,0x18,0x18,0x18,0x00],
    [0x7E,0x06,0x0C,0x18,0x30,0x60,0x7E,0x00],
    [0x3C,0x30,0x30,0x30,0x30,0x30,0x3C,0x00],
    [0x40,0x60,0x30,0x18,0x0C,0x06,0x02,0x00],
    [0x3C,0x0C,0x0C,0x0C,0x0C,0x0C,0x3C,0x00],
    [0x18,0x3C,0x66,0x00,0x00,0x00,0x00,0x00],
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0xFF],
    [0x30,0x18,0x18,0x00,0x00,0x00,0x00,0x00],
    [0x00,0x00,0x3C,0x06,0x3E,0x66,0x3E,0x00],
    [0x00,0x60,0x7C,0x66,0x66,0x66,0x7C,0x00],
    [0x00,0x00,0x3C,0x66,0x60,0x66,0x3C,0x00],
    [0x00,0x06,0x3E,0x66,0x66,0x66,0x3E,0x00],
    [0x00,0x00,0x3C,0x66,0x7E,0x60,0x3C,0x00],
    [0x00,0x0E,0x18,0x3E,0x18,0x18,0x18,0x00],
    [0x00,0x00,0x3E,0x66,0x66,0x3E,0x06,0x7C],
    [0x00,0x60,0x7C,0x66,0x66,0x66,0x66,0x00],
    [0x18,0x00,0x38,0x18,0x18,0x18,0x3C,0x00],
    [0x06,0x00,0x06,0x06,0x06,0x66,0x3C,0x00],
    [0x00,0x60,0x6C,0x78,0x6C,0x6C,0x6C,0x00],
    [0x00,0x38,0x18,0x18,0x18,0x18,0x3C,0x00],
    [0x00,0x00,0x66,0x7F,0x7F,0x66,0x66,0x00],
    [0x00,0x00,0x7C,0x66,0x66,0x66,0x66,0x00],
    [0x00,0x00,0x3C,0x66,0x66,0x66,0x3C,0x00],
    [0x00,0x00,0x7C,0x66,0x66,0x7C,0x60,0x60],
    [0x00,0x00,0x3E,0x66,0x66,0x3E,0x06,0x07],
    [0x00,0x00,0x6C,0x7E,0x60,0x60,0x60,0x00],
    [0x00,0x00,0x3E,0x60,0x3C,0x06,0x7C,0x00],
    [0x00,0x18,0x7E,0x18,0x18,0x18,0x0E,0x00],
    [0x00,0x00,0x66,0x66,0x66,0x66,0x3E,0x00],
    [0x00,0x00,0x66,0x66,0x66,0x3C,0x18,0x00],
    [0x00,0x00,0x66,0x66,0x7F,0x7F,0x66,0x00],
    [0x00,0x00,0x66,0x3C,0x18,0x3C,0x66,0x00],
    [0x00,0x00,0x66,0x66,0x66,0x3E,0x06,0x7C],
    [0x00,0x00,0x7E,0x0C,0x18,0x30,0x7E,0x00],
    [0x0C,0x18,0x18,0x70,0x18,0x18,0x0C,0x00],
    [0x18,0x18,0x18,0x18,0x18,0x18,0x18,0x00],
    [0x30,0x18,0x18,0x0E,0x18,0x18,0x30,0x00],
    [0x3E,0x63,0x00,0x00,0x00,0x00,0x00,0x00],
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00],
];

pub fn g2d_char_hanzi(buf: *mut u8, w: usize, x: usize, y: usize, cp: u32, color: u32) {
    let count = UNICODE_TABLE.len() / 4;
    let mut lo = 0usize;
    let mut hi = count;
    while lo < hi {
        let mid = (lo + hi) / 2;
        let v = u32::from_le_bytes([
            UNICODE_TABLE[mid * 4],
            UNICODE_TABLE[mid * 4 + 1],
            UNICODE_TABLE[mid * 4 + 2],
            UNICODE_TABLE[mid * 4 + 3],
        ]);
        if v == cp {
            let off = mid * 32;
            unsafe {
                for row in 0..16 {
                    let b0 = FONT16[off + row * 2];
                    let b1 = FONT16[off + row * 2 + 1];
                    for col in 0..8 {
                        if b0 & (0x80 >> col) != 0 {
                            px(buf, w, w, x + col, y + row, color);
                        }
                    }
                    for col in 0..8 {
                        if b1 & (0x80 >> col) != 0 {
                            px(buf, w, w, x + 8 + col, y + row, color);
                        }
                    }
                }
            }
            return;
        } else if v < cp {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
}

pub fn g2d_text_utf8(buf: *mut u8, w: usize, mut x: usize, y: usize, text: &[u8], color: u32) {
    let mut i = 0;
    while i < text.len() {
        let b = text[i];
        if b < 0x80 {
            g2d_char(buf, w, x, y, b, color);
            x += 8;
            i += 1;
        } else if i + 2 < text.len() && b >= 0xE0 {
            let cp = ((b as u32 & 0x0F) << 12)
                | ((text[i + 1] as u32 & 0x3F) << 6)
                | (text[i + 2] as u32 & 0x3F);
            g2d_char_hanzi(buf, w, x, y + 4, cp, color);
            x += 16;
            i += 3;
        } else {
            i += 1;
        }
    }
}

pub fn g2d_char(buf: *mut u8, w: usize, x: usize, y: usize, ch: u8, color: u32) {
    if ch < 32 || ch > 127 {
        return;
    }
    let glyph = FONT8X8[(ch - 32) as usize];
    for row in 0..8 {
        let bits = glyph[row];
        for col in 0..8 {
            if (bits >> (7 - col)) & 1 != 0 {
                unsafe { px(buf, w, w, x + col, y + row, color) }
            }
        }
    }
}
