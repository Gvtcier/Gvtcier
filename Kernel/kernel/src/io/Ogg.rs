use alloc::vec::Vec;
use super::Flac::BitReader;

pub struct OggInfo {
    pub sample_rate: u32,
    pub channels: u8,
    pub blocksize_0: u32,
    pub blocksize_1: u32,
}

pub struct Codebook {
    pub dimensions: u16,
    pub entries: u32,
    pub lens: Vec<u8>,
}

pub struct Floor1Config {
    pub partitions: u8,
    pub class_list: Vec<u8>,
    pub class_dim: Vec<u8>,
    pub subclasses: Vec<u8>,
    pub masterbook: Vec<i16>,
    pub subclass_books: Vec<Vec<i16>>,
    pub multiplier: u8,
    pub rangebits: u8,
    pub x_list: Vec<u32>,
}

pub struct ResidueConfig {
    pub rtype: u8,
    pub begin: u32,
    pub end: u32,
    pub partition_size: u32,
    pub classifications: u8,
    pub classbook: u8,
    pub cascade: Vec<u8>,
    pub books: Vec<i16>,
}

pub struct VorbisSetup {
    pub codebooks: Vec<Codebook>,
    pub floors: Vec<Floor1Config>,
    pub residues: Vec<ResidueConfig>,
    pub mappings: Vec<u8>,
    pub modes: Vec<u8>,
}

pub fn codebook_decode(r: &mut BitReader, cb: &Codebook) -> u32 {
    let mut code = 0u32;
    for len in 1..=32u8 {
        code = (code << 1) | r.read(1);
        let mut idx = 0u32;
        for (i, &l) in cb.lens.iter().enumerate() {
            if l == len {
                if idx == code {
                    return i as u32;
                }
                idx += 1;
            }
        }
    }
    0
}

pub fn decode_floor1(r: &mut BitReader, fc: &Floor1Config, books: &[Codebook]) -> Option<Vec<i32>> {
    let flag = r.read(1);
    if flag == 0 {
        return None;
    }
    let mut y: Vec<i32> = Vec::new();
    let mut x: Vec<i32> = Vec::new();
    let mut x_idx = 0usize;
    for &c in &fc.class_list {
        let dim = fc.class_dim[c as usize] as usize;
        let sub = fc.subclasses[c as usize] as usize;
        let mb = fc.masterbook[c as usize];
        let mut vals: Vec<i32> = Vec::new();
        if sub == 0 && mb < 0 {
            for _ in 0..dim {
                vals.push(r.read(fc.rangebits) as i32);
            }
        } else if mb >= 0 {
            let idx = codebook_decode(r, &books[mb as usize]);
            let cb = &books[mb as usize];
            for k in 0..cb.dimensions as usize {
                vals.push(((idx >> (k * 8)) & 0xFF) as i32);
            }
        } else {
            let sb = r.read(sub as u8);
            let book = fc.subclass_books[c as usize][sb as usize];
            if book >= 0 {
                let idx = codebook_decode(r, &books[book as usize]);
                let cb = &books[book as usize];
                for k in 0..cb.dimensions as usize {
                    vals.push(((idx >> (k * 8)) & 0xFF) as i32);
                }
            } else {
                for _ in 0..dim {
                    vals.push(r.read(fc.rangebits) as i32);
                }
            }
        }
        for v in vals {
            if x_idx < fc.x_list.len() {
                x.push(fc.x_list[x_idx] as i32);
            } else {
                x.push(0);
            }
            x_idx += 1;
            y.push(v);
        }
    }
    Some(y)
}

pub fn decode_residue(
    r: &mut BitReader,
    rc: &ResidueConfig,
    books: &[Codebook],
    chs: usize,
    out: &mut [Vec<i32>],
) {
    let total = (rc.end - rc.begin) as usize;
    if total == 0 || rc.partition_size == 0 {
        return;
    }
    let partitions = total / rc.partition_size as usize;
    let classbook = &books[rc.classbook as usize];
    for p in 0..partitions {
        for c in 0..chs {
            let cls = codebook_decode(r, classbook) as usize;
            let sub = r.read(1) as usize;
            let bi = cls * 8 + sub;
            if bi < rc.books.len() {
                let book = rc.books[bi];
                if book >= 0 {
                    let idx = codebook_decode(r, &books[book as usize]);
                    out[c].push(idx as i32);
                }
            }
        }
    }
}

pub fn fast_cos(x: f32) -> f32 {
    let pi = core::f32::consts::PI;
    let mut v = x % (2.0 * pi);
    let mut sign = 1.0f32;
    if v > pi {
        v -= pi;
        sign = -1.0;
    }
    if v > pi / 2.0 {
        v = pi - v;
    }
    let v2 = v * v;
    let v4 = v2 * v2;
    sign * (1.0 - v2 / 2.0 + v4 / 24.0 - v4 * v2 / 720.0 + v4 * v4 / 40320.0)
}

pub fn imdct(n: usize, freq: &[f32], out: &mut [f32]) {
    let inv_n = 2.0 / n as f32;
    for i in 0..n {
        let mut s = 0.0f32;
        for k in 0..n / 2 {
            let angle = core::f32::consts::PI / n as f32
                * (2.0 * (i as f32 + 0.5 + n as f32 / 4.0) * (k as f32 + 0.5));
            s += freq[k] * fast_cos(angle);
        }
        out[i] = s * inv_n;
    }
}

pub fn decode_packet(r: &mut BitReader, setup: &VorbisSetup, channels: u8) -> Option<Vec<i16>> {
    let packet_type = r.read(1);
    if packet_type != 0 {
        return None;
    }
    let mode_bits = (setup.modes.len() as u32).next_power_of_two().trailing_zeros() as u8;
    let mode = r.read(mode_bits) as usize;
    if mode >= setup.modes.len() {
        return None;
    }
    let blockflag = setup.modes[mode] != 0;
    let n = if blockflag { 2048 } else { 256 };
    let ch = channels as usize;
    let mut floors = Vec::new();
    for _ in 0..ch {
        floors.push(decode_floor1(r, &setup.floors[0], &setup.codebooks));
    }
    let mut resid: Vec<Vec<i32>> = Vec::new();
    for _ in 0..ch {
        resid.push(Vec::new());
    }
    decode_residue(r, &setup.residues[0], &setup.codebooks, ch, &mut resid);
    let mut pcm = Vec::new();
    for c in 0..ch {
        let mut freq = Vec::new();
        freq.resize(n / 2, 0.0f32);
        for i in 0..n / 2 {
            freq[i] = resid[c].get(i).copied().unwrap_or(0) as f32;
        }
        let mut out = Vec::new();
        out.resize(n, 0.0f32);
        imdct(n, &freq, &mut out);
        for s in &out {
            pcm.push((s.clamp(-1.0, 1.0) * 32767.0) as i16);
        }
    }
    let _ = floors;
    Some(pcm)
}

pub struct OggPage<'a> {
    pub data: &'a [u8],
    pub body: &'a [u8],
}

pub fn ogg_pages(data: &[u8]) -> Option<Vec<OggPage<'_>>> {
    let mut pages = Vec::new();
    let mut off = 0;
    while off + 27 < data.len() {
        if &data[off..off + 4] == b"OggS" {
            let seg_count = data[off + 26] as usize;
            if off + 27 + seg_count <= data.len() {
                let mut body_len = 0usize;
                for i in 0..seg_count {
                    body_len += data[off + 27 + i] as usize;
                }
                let total = 27 + seg_count + body_len;
                if off + total <= data.len() {
                    pages.push(OggPage {
                        data: &data[off..off + total],
                        body: &data[off + 27 + seg_count..off + total],
                    });
                    off += total;
                    continue;
                }
            }
        }
        off += 1;
    }
    if pages.is_empty() {
        None
    } else {
        Some(pages)
    }
}

pub fn info(data: &[u8]) -> Option<OggInfo> {
    let pages = ogg_pages(data)?;
    for p in pages {
        let body = p.body;
        if body.len() >= 30 && body[0] == 0x01 && &body[1..7] == b"vorbis" {
            let channels = body[11] as u8;
            let sample_rate = u32::from_le_bytes([body[12], body[13], body[14], body[15]]);
            return Some(OggInfo {
                sample_rate,
                channels,
                blocksize_0: 256,
                blocksize_1: 2048,
            });
        }
    }
    None
}

pub fn setup(data: &[u8]) -> Option<VorbisSetup> {
    let pages = ogg_pages(data)?;
    for p in pages {
        let body = p.body;
        if body.len() >= 7 && body[0] == 0x05 && &body[1..7] == b"vorbis" {
            let mut r = BitReader::new(body);
            for _ in 0..7 {
                r.read(8);
            }
            r.read(32);
            r.read(8);
            r.read(32);
            r.read(32);
            r.read(32);
            r.read(32);
            r.read(4);
            r.read(4);
            r.read(1);
            let codebook_count = r.read(8) + 1;
            let mut books = Vec::new();
            for _ in 0..codebook_count {
                let sync = r.read(24);
                if sync != 0x564342 {
                    break;
                }
                let dimensions = r.read(16) as u16;
                let entries = r.read(24);
                let ordered = r.read(1);
                let mut lens: Vec<u8> = Vec::new();
                if ordered == 0 {
                    let sparse = r.read(1);
                    for _ in 0..entries {
                        lens.push(r.read(5) as u8);
                    }
                    let _ = sparse;
                } else {
                    let mut il = r.read(5) as u8;
                    let mut n = 1u32;
                    while n < entries {
                        let inc = r.read(5);
                        for _ in 0..(1usize << inc) {
                            lens.push(il);
                            n += 1;
                            if n >= entries {
                                break;
                            }
                        }
                        il += 1;
                    }
                }
                let lookup_type = r.read(4);
                if lookup_type > 0 {
                    r.read(32);
                    r.read(32);
                    r.read(32);
                    let value_count = r.read(4) + 1;
                    r.read(8);
                    for _ in 0..value_count {
                        r.read(8);
                    }
                }
                books.push(Codebook { dimensions, entries, lens });
            }
            let floor_count = r.read(6);
            let mut floors = Vec::new();
            for _ in 0..floor_count {
                let ftype = r.read(16);
                if ftype == 0 {
                    r.read(8);
                    r.read(16);
                    r.read(16);
                    r.read(6);
                    r.read(8);
                    let num_books = r.read(4);
                    for _ in 0..(num_books + 1) {
                        r.read(8);
                    }
                    floors.push(Floor1Config {
                        partitions: 0,
                        class_list: Vec::new(),
                        class_dim: Vec::new(),
                        subclasses: Vec::new(),
                        masterbook: Vec::new(),
                        subclass_books: Vec::new(),
                        multiplier: 0,
                        rangebits: 0,
                        x_list: Vec::new(),
                    });
                } else {
                    let partitions = r.read(5);
                    let mut class_list = Vec::new();
                    let mut max_class = 0u8;
                    for _ in 0..partitions {
                        let c = r.read(4) as u8;
                        class_list.push(c);
                        if c > max_class {
                            max_class = c;
                        }
                    }
                    let classes = max_class + 1;
                    let mut class_dim = Vec::new();
                    for _ in 0..classes {
                        class_dim.push(r.read(3) as u8);
                    }
                    let mut subclasses = Vec::new();
                    for _ in 0..classes {
                        subclasses.push(r.read(2) as u8);
                    }
                    let mut masterbook = Vec::new();
                    for _ in 0..classes {
                        masterbook.push(r.read(8) as i16);
                    }
                    let mut subclass_books = Vec::new();
                    for c in 0..classes {
                        let mut books = Vec::new();
                        for _ in 0..(1usize << subclasses[c as usize]) {
                            books.push(r.read(8) as i16);
                        }
                        subclass_books.push(books);
                    }
                    let multiplier = r.read(2) as u8;
                    let rangebits = r.read(4);
                    let mut x_list: Vec<u32> = Vec::new();
                    x_list.push(0);
                    x_list.push(1u32 << rangebits);
                    for &c in &class_list {
                        for _ in 0..(1usize << class_dim[c as usize]) {
                            let x = r.read(rangebits as u8);
                            x_list.push(x);
                        }
                    }
                    floors.push(Floor1Config {
                        partitions: partitions as u8,
                        class_list,
                        class_dim,
                        subclasses,
                        masterbook,
                        subclass_books,
                        multiplier,
                        rangebits: rangebits as u8,
                        x_list,
                    });
                }
            }
            let residue_count = r.read(6);
            let mut residues = Vec::new();
            for _ in 0..residue_count {
                let rtype = r.read(16) as u8;
                let begin = r.read(24);
                let end = r.read(24);
                let partition_size = r.read(24);
                let classifications = r.read(6);
                let classbook = r.read(8);
                let mut cascade = Vec::new();
                for _ in 0..classifications {
                    let low = r.read(3);
                    let high = r.read(1);
                    cascade.push(((high as u8) << 5) | (low as u8));
                }
                let mut books = Vec::new();
                for c in 0..classifications {
                    for j in 0..8u8 {
                        if cascade[c as usize] >> j & 1 != 0 {
                            books.push(r.read(8) as i16);
                        }
                    }
                }
                residues.push(ResidueConfig {
                    rtype,
                    begin,
                    end,
                    partition_size,
                    classifications: classifications as u8,
                    classbook: classbook as u8,
                    cascade,
                    books,
                });
            }
            let mapping_count = r.read(6);
            let mut mappings = Vec::new();
            for _ in 0..mapping_count {
                let mtype = r.read(16);
                let submaps = r.read(4);
                if submaps > 0 {
                    let coupling_steps = r.read(8);
                    for _ in 0..coupling_steps {
                        r.read(8);
                        r.read(8);
                    }
                }
                let coupling_bits = if submaps > 0 { 1 } else { 0 };
                let channels = 0u32;
                let _ = (coupling_bits, channels);
                let mut mux_bits = 0u8;
                while (1u32 << mux_bits) < submaps {
                    mux_bits += 1;
                }
                let _ = mux_bits;
                for _ in 0..(if submaps > 0 { submaps } else { 1 }) {
                    r.read(8);
                    r.read(8);
                }
                mappings.push(mtype as u8);
            }
            let mode_count = r.read(6);
            let mut modes = Vec::new();
            for _ in 0..mode_count {
                let blockflag = r.read(1);
                r.read(16);
                r.read(16);
                r.read(8);
                modes.push(blockflag as u8);
            }
            r.read(1);
            return Some(VorbisSetup {
                codebooks: books,
                floors,
                residues,
                mappings,
                modes,
            });
        }
    }
    None
}
