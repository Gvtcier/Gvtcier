pub const MAX_CANVAS: usize = 16;

#[repr(C)]
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub buf: usize,
}

const CANVAS_NONE: Canvas = Canvas { w: 0, h: 0, buf: 0 };

static mut CANVASES: [Canvas; MAX_CANVAS] = [CANVAS_NONE; MAX_CANVAS];

pub fn canvas_create(w: usize, h: usize, buf: usize) -> u32 {
    if w == 0 || h == 0 || buf == 0 {
        return 0xFFFFFFFF;
    }
    unsafe {
        for i in 0..MAX_CANVAS {
            if CANVASES[i].w == 0 {
                CANVASES[i] = Canvas { w, h, buf };
                return i as u32;
            }
        }
    }
    0xFFFFFFFF
}

pub fn canvas_destroy(id: u32) {
    unsafe {
        if (id as usize) < MAX_CANVAS {
            CANVASES[id as usize] = CANVAS_NONE;
        }
    }
}

pub fn canvas_buf(id: u32) -> usize {
    unsafe {
        if (id as usize) < MAX_CANVAS {
            return CANVASES[id as usize].buf;
        }
    }
    0
}

pub fn canvas_w(id: u32) -> usize {
    unsafe {
        if (id as usize) < MAX_CANVAS {
            return CANVASES[id as usize].w;
        }
    }
    0
}

pub fn canvas_h(id: u32) -> usize {
    unsafe {
        if (id as usize) < MAX_CANVAS {
            return CANVASES[id as usize].h;
        }
    }
    0
}

pub fn canvas_count() -> u32 {
    unsafe {
        let mut n: u32 = 0;
        for i in 0..MAX_CANVAS {
            if CANVASES[i].w != 0 {
                n += 1;
            }
        }
        n
    }
}

pub fn compose(id: u32, x: usize, y: usize) {
    unsafe {
        if (id as usize) < MAX_CANVAS {
            let c = &CANVASES[id as usize];
            if c.w != 0 {
                crate::io::Fb::blit(c.buf as *const u8, c.w, c.h, x, y);
            }
        }
    }
}

pub fn compose_clip(id: u32, x: usize, y: usize, fw: usize, fh: usize) {
    unsafe {
        if (id as usize) < MAX_CANVAS {
            let c = &CANVASES[id as usize];
            if c.w != 0 {
                let cw = if x + c.w > fw { fw.saturating_sub(x) } else { c.w };
                let ch = if y + c.h > fh { fh.saturating_sub(y) } else { c.h };
                if cw > 0 && ch > 0 {
                    crate::io::Fb::blit(c.buf as *const u8, cw, ch, x, y);
                }
            }
        }
    }
}
