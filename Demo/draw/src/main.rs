#![no_std]
#![no_main]

mod Common;

use Common::*;

include!("../../../GvFont/gvfont_data.rs");

const W: usize = 1920;
const H: usize = 1080;
const SCALE: usize = 3;
const GAP: usize = 6;
const BG: u32 = 0xFF2030A0;
const FG: u32 = 0xFFFFFFFF;

#[no_mangle]
static mut FBUF: [u8; W * H * 4] = [0; W * H * 4];

unsafe fn set_px(buf: &mut [u8], x: usize, y: usize, color: u32) {
    if x >= W || y >= H {
        return;
    }
    let off = (y * W + x) * 4;
    buf[off] = color as u8;
    buf[off + 1] = (color >> 8) as u8;
    buf[off + 2] = (color >> 16) as u8;
    buf[off + 3] = (color >> 24) as u8;
}

unsafe fn fill_bg(buf: &mut [u8]) {
    let mut y = 0;
    while y < H {
        let mut x = 0;
        while x < W {
            set_px(buf, x, y, BG);
            x += 1;
        }
        y += 1;
    }
}

fn ascii_to_gv(c: u8) -> u8 {
    if 0x41 <= c && c <= 0x5A {
        c - 0x21
    } else if 0x61 <= c && c <= 0x7A {
        c - 0x27
    } else {
        c
    }
}

unsafe fn draw_skin_char(buf: &mut [u8], ch: u8, x0: usize, y0: usize) -> usize {
    let gv = ascii_to_gv(ch);
    if let Some(bits) = gv2280_bitmap(gv) {
        let mut r = 0;
        while r < 10 {
            let mut c = 0;
            while c < 10 {
                if bits[r * 10 + c] == 1 {
                    let mut dy = 0;
                    while dy < SCALE {
                        let mut dx = 0;
                        while dx < SCALE {
                            set_px(buf, x0 + c * SCALE + dx, y0 + r * SCALE + dy, FG);
                            dx += 1;
                        }
                        dy += 1;
                    }
                }
                c += 1;
            }
            r += 1;
        }
        x0 + 10 * SCALE + GAP
    } else if ch == b' ' {
        x0 + 10 * SCALE + GAP + 12
    } else {
        x0 + 10 * SCALE + GAP
    }
}

#[no_mangle]
extern "C" fn user_main() -> ! {
    unsafe {
        let buf: &mut [u8] = &mut FBUF;
        fill_bg(buf);
        let line1 = b"Welcome to Gvtcier Kernel!";
        let mut x = 60usize;
        let mut i = 0;
        while i < line1.len() {
            x = draw_skin_char(buf, line1[i], x, 60);
            i += 1;
        }
        let canvas = g2d_canvas_create(W as u64, H as u64, buf.as_ptr() as u64);
        g2d_canvas_map(canvas);
        g2d_text(canvas, 60, 300, FG, b"Welcome to Gvtcier Kernel!\0");
        g2d_text(canvas, 60, 500, FG, "欢迎来到 Gvtcier 内核!\0".as_bytes());
        g2d_compose(canvas, 0, 0);
        g2d_flush();
        loop {
            core::hint::spin_loop();
        }
    }
}
