include!("gv2280_tables.rs");

const GVSYM: &[u8] = b" !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

pub fn ascii_to_gv(b: u8) -> u8 {
    match b {
        0x41..=0x5A => 0x20u8 + (b - 0x41u8),
        0x61..=0x7A => 0x3Au8 + (b - 0x61u8),
        0x30..=0x39 => 0x54u8 + (b - 0x30u8),
        _ => {
            let mut i = 0;
            while i < GVSYM.len() {
                if GVSYM[i] == b {
                    return 0x5Eu8 + i as u8;
                }
                i += 1;
            }
            b
        }
    }
}

pub fn gv_to_ascii(b: u8) -> u8 {
    if b < 0x20u8 {
        return b;
    }
    match b {
        0x20..=0x39 => 0x41u8 + (b - 0x20u8),
        0x3A..=0x53 => 0x61u8 + (b - 0x3Au8),
        0x54..=0x5D => 0x30u8 + (b - 0x54u8),
        _ => {
            let idx = (b - 0x5Eu8) as usize;
            if idx < GVSYM.len() {
                GVSYM[idx]
            } else {
                b
            }
        }
    }
}

fn hanzi_utf8_at(i: usize) -> [u8; 3] {
    let bit = i * 15usize;
    let mut v: u32 = 0;
    let mut k = 0;
    while k < 15 {
        let idx = bit + k;
        let byte = idx >> 3usize;
        let off = 7usize - (idx & 7usize);
        v = (v << 1) | (((HANZI_PACK[byte] >> off) & 1u8) as u32);
        k += 1;
    }
    [
        0xE4u8 + (v >> 12) as u8,
        0x80u8 + ((v >> 6) & 0x3F) as u8,
        0x80u8 + (v & 0x3F) as u8,
    ]
}

pub fn hanzi_code(ch: &[u8; 3]) -> Option<usize> {
    let mut i = 0;
    while i < 6763 {
        let h = hanzi_utf8_at(i);
        if h[0] == ch[0] && h[1] == ch[1] && h[2] == ch[2] {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub fn utf8_to_gv(src: &[u8], out: &mut [u8]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < src.len() {
        let b = src[i];
        if b < 0x80u8 {
            if n < out.len() {
                out[n] = ascii_to_gv(b);
                n += 1;
            }
            i += 1;
        } else if b >= 0xE0u8 && i + 2 < src.len() {
            let ch = [b, src[i + 1], src[i + 2]];
            if let Some(code) = hanzi_code(&ch) {
                if n + 1 < out.len() {
                    out[n] = 0x80u8 + (code >> 8) as u8;
                    out[n + 1] = code as u8;
                    n += 2;
                }
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    n
}

pub fn gv_to_utf8(src: &[u8], out: &mut [u8]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < src.len() {
        let b = src[i];
        if b < 0x80u8 {
            if n < out.len() {
                out[n] = gv_to_ascii(b);
                n += 1;
            }
            i += 1;
        } else if i + 1 < src.len() {
            let code = ((b as usize - 0x80usize) << 8) | src[i + 1] as usize;
            if code < 6763 && n + 2 < out.len() {
                let h = hanzi_utf8_at(code);
                out[n] = h[0];
                out[n + 1] = h[1];
                out[n + 2] = h[2];
                n += 3;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    n
}
