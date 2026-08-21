use alloc::vec::Vec;

pub struct FlacInfo {
    pub sample_rate: u32,
    pub channels: u8,
    pub bits: u8,
    pub blocksize: u32,
}

pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    cur: u8,
    bit: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BitReader { data, pos: 0, cur: 0, bit: 0 }
    }
    pub fn read(&mut self, n: u8) -> u32 {
        let mut v = 0u32;
        for _ in 0..n {
            v = (v << 1) | self.read1();
        }
        v
    }
    fn read1(&mut self) -> u32 {
        if self.bit == 0 {
            self.cur = self.data.get(self.pos).copied().unwrap_or(0);
            self.pos += 1;
            self.bit = 8;
        }
        self.bit -= 1;
        ((self.cur >> self.bit) & 1) as u32
    }
    pub fn align_byte(&mut self) {
        self.bit = 0;
    }
    pub fn pos(&self) -> usize {
        self.pos
    }
}

pub fn sign_extend(v: u32, n: u8) -> i32 {
    if n >= 32 {
        return v as i32;
    }
    let m = 1u32 << (n - 1);
    if v & m != 0 {
        (v | !((1u32 << n) - 1)) as i32
    } else {
        v as i32
    }
}

fn rice_decode(r: &mut BitReader, param: u8, count: usize, out: &mut Vec<i32>) -> bool {
    for _ in 0..count {
        let mut q = 0u32;
        while r.read(1) == 0 {
            q += 1;
            if q > 32 {
                return false;
            }
        }
        let rem = if param > 0 { r.read(param) } else { 0 };
        let mut v = (q << param) | rem;
        if v & 1 != 0 {
            v = !(v >> 1);
        } else {
            v >>= 1;
        }
        out.push(v as i32);
    }
    true
}

fn decode_constant(r: &mut BitReader, bits: u8, count: usize, out: &mut Vec<i32>) -> bool {
    let v = sign_extend(r.read(bits), bits);
    for _ in 0..count {
        out.push(v);
    }
    true
}

fn decode_verbatim(r: &mut BitReader, bits: u8, count: usize, out: &mut Vec<i32>) -> bool {
    for _ in 0..count {
        out.push(sign_extend(r.read(bits), bits));
    }
    true
}

fn decode_residual(r: &mut BitReader, blocksize: usize, order: usize, out: &mut Vec<i32>) -> bool {
    let method = r.read(2);
    let partition_order = r.read(4);
    let partitions = 1usize << partition_order;
    let total = blocksize - order;
    let base = total / partitions;
    let rem = total % partitions;
    for p in 0..partitions {
        let psize = if p == partitions - 1 { base + rem } else { base };
        let mut param = r.read(4);
        if method == 1 {
            param = (param << 5) | r.read(5);
        } else if param == 15 {
            param += r.read(5);
        }
        if !rice_decode(r, param as u8, psize, out) {
            return false;
        }
    }
    true
}

fn decode_fixed(r: &mut BitReader, bits: u8, order: usize, blocksize: usize, out: &mut Vec<i32>) -> bool {
    for _ in 0..order {
        out.push(sign_extend(r.read(bits), bits));
    }
    let mut residual: Vec<i32> = Vec::new();
    if !decode_residual(r, blocksize, order, &mut residual) {
        return false;
    }
    for (i, &res) in residual.iter().enumerate() {
        let n = order + i;
        let pred = match order {
            1 => out[n - 1],
            2 => 2 * out[n - 1] - out[n - 2],
            3 => 3 * out[n - 1] - 3 * out[n - 2] + out[n - 3],
            4 => 4 * out[n - 1] - 6 * out[n - 2] + 4 * out[n - 3] - out[n - 4],
            _ => 0,
        };
        out.push(pred + res);
    }
    true
}

fn decode_lpc(r: &mut BitReader, bits: u8, order: usize, blocksize: usize, out: &mut Vec<i32>) -> bool {
    for _ in 0..order {
        out.push(sign_extend(r.read(bits), bits));
    }
    let precision = r.read(4) + 1;
    let shift2 = r.read(5);
    let mut coef: Vec<i32> = Vec::new();
    for _ in 0..order {
        coef.push(sign_extend(r.read(precision as u8), precision as u8));
    }
    let mut residual: Vec<i32> = Vec::new();
    if !decode_residual(r, blocksize, order, &mut residual) {
        return false;
    }
    for (i, &res) in residual.iter().enumerate() {
        let n = order + i;
        let mut acc: i64 = 0;
        for k in 0..order {
            acc += (coef[k] as i64) * (out[n - 1 - k] as i64);
        }
        let pred = (acc >> shift2) as i32;
        out.push(pred + res);
    }
    true
}

pub fn info(data: &[u8]) -> Option<FlacInfo> {
    if data.len() < 4 || &data[0..4] != b"fLaC" {
        return None;
    }
    let mut off = 4usize;
    loop {
        if off + 4 >= data.len() {
            return None;
        }
        let h = data[off];
        let last = h & 0x80 != 0;
        let btype = h & 0x7F;
        let len = ((data[off + 1] as u32) << 16) | ((data[off + 2] as u32) << 8) | data[off + 3] as u32;
        if btype == 0 {
            if off + 4 + 18 > data.len() {
                return None;
            }
            let b = &data[off + 4..off + 4 + 18];
            let blocksize = ((b[2] as u32) << 8) | b[3] as u32;
            let sr_bits = ((b[8] as u32) << 12) | ((b[9] as u32) << 4) | ((b[10] as u32) >> 4);
            let ch_bits = (b[10] >> 1) & 0x7;
            let bits = ((b[10] & 1) << 4) | (b[11] >> 4);
            return Some(FlacInfo {
                sample_rate: sr_bits,
                channels: (ch_bits + 1) as u8,
                bits: (bits + 1) as u8,
                blocksize,
            });
        }
        off += 4 + len as usize;
        if last {
            break;
        }
    }
    None
}

pub fn decode(data: &[u8], out: &mut Vec<i16>) -> Option<FlacInfo> {
    let fi = info(data)?;
    let mut off = 4usize;
    loop {
        if off + 4 >= data.len() {
            return None;
        }
        let h = data[off];
        let last = h & 0x80 != 0;
        let len = ((data[off + 1] as u32) << 16) | ((data[off + 2] as u32) << 8) | data[off + 3] as u32;
        off += 4 + len as usize;
        if last {
            break;
        }
    }
    while off + 2 < data.len() {
        if data[off] != 0xFF || (data[off + 1] & 0xFC) != 0xF8 {
            off += 1;
            continue;
        }
        let mut r = BitReader::new(&data[off..]);
        r.read(14);
        r.read(1);
        let blocksize_code = r.read(4);
        let _sr_code = r.read(4);
        let ch_assign = r.read(4);
        let bps_code = r.read(3);
        r.read(1);
        r.read(8);
        r.read(8);
        let blocksize: u32 = if blocksize_code == 0 {
            256
        } else if blocksize_code == 6 {
            4608
        } else if blocksize_code == 7 {
            0
        } else {
            1 << blocksize_code
        };
        let channels: u8 = if ch_assign < 8 { (ch_assign + 1) as u8 } else { 2 };
        let bits: u8 = match bps_code {
            0 => 8,
            1 => 12,
            2 => 16,
            3 => 20,
            4 => 24,
            5 => 32,
            _ => 16,
        };
        let mut chs: Vec<Vec<i32>> = Vec::new();
        let mut ok = true;
        for _ in 0..channels {
            r.read(1);
            let sfbps = r.read(1);
            let sf_type = r.read(6);
            let shift = r.read(sfbps as u8);
            let mut samples: Vec<i32> = Vec::new();
            match sf_type {
                0 => {
                    ok = decode_constant(&mut r, bits, blocksize as usize, &mut samples);
                }
                1 => {
                    ok = decode_verbatim(&mut r, bits, blocksize as usize, &mut samples);
                }
                8..=12 => {
                    ok = decode_fixed(
                        &mut r,
                        bits,
                        (sf_type - 8) as usize,
                        blocksize as usize,
                        &mut samples,
                    );
                }
                32..=63 => {
                    ok = decode_lpc(
                        &mut r,
                        bits,
                        (sf_type - 31) as usize,
                        blocksize as usize,
                        &mut samples,
                    );
                }
                _ => {
                    ok = false;
                }
            }
            if !ok {
                break;
            }
            chs.push(samples);
        }
        if !ok || chs.len() != channels as usize {
            break;
        }
        for i in 0..blocksize as usize {
            for c in 0..channels as usize {
                if let Some(ch) = chs.get(c) {
                    if let Some(&s) = ch.get(i) {
                        let v = s.clamp(-32768, 32767) as i16;
                        out.push(v);
                    }
                }
            }
        }
        r.align_byte();
        let consumed = r.pos + 2;
        if consumed <= 2 {
            break;
        }
        off += consumed;
    }
    Some(fi)
}
