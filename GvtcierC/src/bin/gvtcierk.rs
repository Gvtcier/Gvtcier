use std::collections::HashMap;
use std::env;
use std::fs;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Sec {
    Text,
    Data,
    Bss,
}

#[derive(Debug)]
enum Item {
    Instr(String),
    Data(Vec<u8>),
    Zero(usize),
    Label(String),
}

fn parse(src: &str) -> Vec<(Sec, Item)> {
    let mut items: Vec<(Sec, Item)> = Vec::new();
    let mut sec = Sec::Text;
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.ends_with(':') {
            items.push((sec, Item::Label(line[..line.len() - 1].to_string())));
            continue;
        }
        if line.starts_with('.') {
            match line {
                ".text" => sec = Sec::Text,
                ".data" => sec = Sec::Data,
                ".bss" => sec = Sec::Bss,
                _ => {
                    if line.starts_with(".intel_syntax") || line.starts_with(".global") {
                        continue;
                    }
                    if let Some(rest) = line.strip_prefix(".zero") {
                        items.push((sec, Item::Zero(rest.trim().parse().unwrap())));
                        continue;
                    }
                    if let Some(rest) = line.strip_prefix(".ascii") {
                        items.push((sec, Item::Data(parse_ascii(rest.trim()))));
                        continue;
                    }
                    if let Some(rest) = line.strip_prefix(".byte") {
                        items.push((sec, Item::Data(vec![rest.trim().parse().unwrap()])));
                        continue;
                    }
                    eprintln!("gvtcierk: unknown directive: {}", line);
                }
            }
            continue;
        }
        items.push((sec, Item::Instr(line.to_string())));
    }
    items
}

fn parse_ascii(s: &str) -> Vec<u8> {
    let s = s.trim();
    if s.len() < 2 {
        return Vec::new();
    }
    let inner = &s[1..s.len() - 1];
    let mut out = Vec::new();
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push(b'\n'),
                Some('\\') => out.push(b'\\'),
                Some('"') => out.push(b'"'),
                Some(x) => out.push(x as u8),
                None => {}
            }
        } else {
            out.push(c as u8);
        }
    }
    out
}

#[derive(Clone, Debug)]
enum Op {
    R { w: u8, reg: u8, ext: bool },
    X { reg: u8, ext: bool },
    M {
        base: Option<(bool, u8)>,
        index: Option<(bool, u8)>,
        scale: u8,
        disp: i32,
        rip: Option<String>,
    },
    I(i64),
}

fn reg_code(name: &str) -> Option<(u8, bool)> {
    let c = match name {
        "rax" => 0,
        "rcx" => 1,
        "rdx" => 2,
        "rbx" => 3,
        "rsp" => 4,
        "rbp" => 5,
        "rsi" => 6,
        "rdi" => 7,
        "r8" => 8,
        "r9" => 9,
        "r10" => 10,
        "r11" => 11,
        "r12" => 12,
        "r13" => 13,
        "r14" => 14,
        "r15" => 15,
        "eax" => 0,
        "ecx" => 1,
        "edx" => 2,
        "ebx" => 3,
        "esp" => 4,
        "ebp" => 5,
        "esi" => 6,
        "edi" => 7,
        "r8d" => 8,
        "r9d" => 9,
        "r10d" => 10,
        "r11d" => 11,
        "r12d" => 12,
        "r13d" => 13,
        "r14d" => 14,
        "r15d" => 15,
        _ => return None,
    };
    Some((c & 7, c >= 8))
}

fn reg8_code(name: &str) -> Option<(u8, bool)> {
    let n = match name {
        "al" => 0, "cl" => 1, "dl" => 2, "bl" => 3, "spl" => 4, "bpl" => 5,
        "sil" => 6, "dil" => 7, "r8b" => 8, "r9b" => 9, "r10b" => 10, "r11b" => 11,
        "r12b" => 12, "r13b" => 13, "r14b" => 14, "r15b" => 15, _ => return None,
    };
    Some((n & 7, n >= 8))
}

fn xmm_code(name: &str) -> Option<(u8, bool)> {
    if let Some(x) = name.strip_prefix("xmm") {
        let n: usize = x.parse().ok()?;
        if n < 16 {
            return Some(((n & 7) as u8, n >= 8));
        }
    }
    None
}

fn parse_op(s: &str) -> Option<Op> {
    let s = s.trim();
    let s = s.strip_prefix("byte ptr ").unwrap_or(s);
    if s.starts_with('[') {
        let inner = s.trim_start_matches('[').trim_end_matches(']');
        if let Some(rest) = inner.strip_prefix("rip + ") {
            return Some(Op::M {
                base: None,
                index: None,
                scale: 0,
                disp: 0,
                rip: Some(rest.trim().to_string()),
            });
        }
        let mut base: Option<(bool, u8)> = None;
        let mut index: Option<(bool, u8)> = None;
        let mut scale: u8 = 0;
        let mut disp: i32 = 0;
        let mut tokens: Vec<(i32, String)> = Vec::new();
        let mut cur = String::new();
        let mut sign = 1i32;
        for ch in inner.chars() {
            if ch == '+' || ch == '-' {
                if !cur.is_empty() {
                    tokens.push((sign, std::mem::take(&mut cur)));
                }
                sign = if ch == '+' { 1 } else { -1 };
            } else {
                cur.push(ch);
            }
        }
        if !cur.is_empty() {
            tokens.push((sign, cur));
        }
        for (sg, tok) in tokens {
            let tok = tok.trim();
            if let Ok(v) = tok.parse::<i32>() {
                disp += sg * v;
                continue;
            }
            if let Some((ix, is)) = tok.split_once('*') {
                if let Some((c, e)) = reg_code(ix.trim()) {
                    index = Some((e, c));
                    scale = is.trim().parse().ok()?;
                }
                continue;
            }
            if let Some((c, e)) = reg_code(tok) {
                if base.is_none() {
                    base = Some((e, c));
                } else if index.is_none() {
                    index = Some((e, c));
                    scale = 0;
                } else {
                    return None;
                }
                continue;
            }
            return None;
        }
        return Some(Op::M {
            base,
            index,
            scale,
            disp,
            rip: None,
        });
    }
    if let Some(x) = reg_code(s) {
        let is32 = matches!(s, "eax" | "ecx" | "edx" | "ebx" | "esp" | "ebp" | "esi" | "edi")
            || s.ends_with('d');
        return Some(Op::R {
            w: if is32 { 1 } else { 2 },
            reg: x.0,
            ext: x.1,
        });
    }
    if let Some(x) = reg8_code(s) {
        return Some(Op::R { w: 0, reg: x.0, ext: x.1 });
    }
    if let Some(x) = xmm_code(s) {
        return Some(Op::X { reg: x.0, ext: x.1 });
    }
    if let Ok(v) = s.parse::<i64>() {
        return Some(Op::I(v));
    }
    None
}

struct Enc {
    bytes: Vec<u8>,
    rex: u8,
    rex_needed: bool,
}

fn enc_push_rex(e: &mut Enc, w: bool, r: bool, x: bool, b: bool) {
    if w || r || x || b {
        e.rex = 0x40 | (if w { 8 } else { 0 }) | (if r { 4 } else { 0 })
            | (if x { 2 } else { 0 })
            | (if b { 1 } else { 0 });
        e.bytes.push(e.rex);
    }
}

fn enc_modrm(e: &mut Enc, mod_: u8, reg: u8, rm: u8) {
    e.bytes.push((mod_ << 6) | ((reg & 7) << 3) | (rm & 7));
}

fn enc_sib(e: &mut Enc, scale: u8, index: u8, base: u8) {
    let sf = match scale {
        8 => 3,
        4 => 2,
        2 => 1,
        _ => 0,
    };
    e.bytes.push((sf << 6) | ((index & 7) << 3) | (base & 7));
}

fn enc_imm(e: &mut Enc, v: i64, n: usize) {
    for i in 0..n {
        e.bytes.push(((v >> (8 * i)) & 0xFF) as u8);
    }
}

fn enc_mem(e: &mut Enc, m: &Op) -> Result<(u8, u8), String> {
    if let Op::M { base, index, scale, disp, rip } = m {
        let base = base.map(|(b, c)| (b, c));
        let index = index.map(|(b, c)| (b, c));
        let scale = *scale;
        let disp = *disp;
        let rip = rip.clone();
        if rip.is_some() {
            return Ok((0, 5));
        }
        match (base, index) {
            (None, None) => return Err("bad mem".into()),
            (Some((be, bc)), Some((ie, ic))) => {
                let (mod_, dispbytes) = if disp == 0 {
                    (0u8, 0)
                } else if (disp as i8) as i32 == disp {
                    (1u8, 1)
                } else {
                    (2u8, 4)
                };
                enc_push_rex(e, false, false, ie, be);
                enc_modrm(e, mod_, 0, 4);
                enc_sib(e, scale, ic, bc);
                if dispbytes == 1 {
                    enc_imm(e, disp as i64, 1);
                } else if dispbytes == 4 {
                    enc_imm(e, disp as i64, 4);
                }
                return Ok((mod_, 4));
            }
            (Some((be, bc)), None) => {
                if bc == 4 {
                    let (mod_, dispbytes) = if disp == 0 {
                        (0u8, 0)
                    } else if (disp as i8) as i32 == disp {
                        (1u8, 1)
                    } else {
                        (2u8, 4)
                    };
                    enc_push_rex(e, false, false, false, be);
                    enc_modrm(e, mod_, 0, 4);
                    enc_sib(e, 0, 4, bc);
                    if dispbytes == 1 {
                        enc_imm(e, disp as i64, 1);
                    } else if dispbytes == 4 {
                        enc_imm(e, disp as i64, 4);
                    }
                    return Ok((mod_, 4));
                }
                if bc == 5 && disp == 0 {
                    enc_push_rex(e, false, false, false, be);
                    enc_modrm(e, 1, 0, 5);
                    enc_imm(e, 0, 1);
                    return Ok((1, 5));
                }
                let (mod_, dispbytes) = if disp == 0 {
                    (0u8, 0)
                } else if (disp as i8) as i32 == disp {
                    (1u8, 1)
                } else {
                    (2u8, 4)
                };
                enc_push_rex(e, false, false, false, be);
                enc_modrm(e, mod_, 0, bc);
                if dispbytes == 1 {
                    enc_imm(e, disp as i64, 1);
                } else if dispbytes == 4 {
                    enc_imm(e, disp as i64, 4);
                }
                return Ok((mod_, bc));
            }
            (None, Some((ie, ic))) => {
                let (mod_, dispbytes) = if disp == 0 {
                    (0u8, 0)
                } else if (disp as i8) as i32 == disp {
                    (1u8, 1)
                } else {
                    (2u8, 4)
                };
                enc_push_rex(e, false, false, ie, false);
                enc_modrm(e, mod_, 0, 4);
                enc_sib(e, scale, ic, 5);
                let dbytes = if dispbytes == 0 { 4 } else { dispbytes };
                enc_imm(e, disp as i64, dbytes);
                return Ok((mod_, 4));
            }
        }
    }
    Err("not mem".into())
}

fn enc_reg_operand(e: &mut Enc, w: bool, reg: u8, ext: bool) {
    let _ = (w, reg, ext);
}

fn encode(text: &str, addr: usize, syms: &HashMap<String, usize>) -> Result<Vec<u8>, String> {
    let mut parts = text.split_whitespace();
    let mne = parts.next().unwrap_or("").to_string();
    let rest: String = parts.collect::<Vec<_>>().join(" ");
    let ops: Vec<Op> = if rest.is_empty() {
        Vec::new()
    } else {
        rest.split(',').filter_map(|s| parse_op(s)).collect()
    };
    let mut e = Enc {
        bytes: Vec::new(),
        rex: 0,
        rex_needed: false,
    };

    macro_rules! need_rex {
        ($w:expr, $r:expr, $b:expr) => {{
            let (ww, rr, bb) = ($w, $r, $b);
            if ww || rr || bb {
                e.bytes.push(0x40 | (if ww { 8 } else { 0 }) | (if rr { 4 } else { 0 }) | (if bb { 1 } else { 0 }));
            }
        }};
    }

    match mne.as_str() {
        "push" => {
            if let Op::R { w: 2, reg, ext } = &ops[0] {
                need_rex!(false, false, *ext);
                e.bytes.push(0x50 + (reg & 7));
            }
        }
        "pop" => {
            if let Op::R { w: 2, reg, ext } = &ops[0] {
                need_rex!(false, false, *ext);
                e.bytes.push(0x58 + (reg & 7));
            }
        }
        "ret" => e.bytes.push(0xC3),
        "leave" => e.bytes.push(0xC9),
        "cdq" => e.bytes.push(0x99),
        "cqo" => e.bytes.extend_from_slice(&[0x48, 0x99]),
        "mov" => {
            let a = &ops[0];
            let b = &ops[1];
            match (a, b) {
                (Op::R { w: 2, reg, ext }, Op::I(v)) => {
                    need_rex!(true, false, *ext);
                    e.bytes.push(0xB8 + (reg & 7));
                    enc_imm(&mut e, *v, 8);
                }
                (Op::R { w: 1, reg, ext }, Op::I(v)) => {
                    need_rex!(false, false, *ext);
                    e.bytes.push(0xB8 + (reg & 7));
                    enc_imm(&mut e, *v, 4);
                }
                (Op::R { w: 2, reg, ext }, Op::R { w: 2, reg: r2, ext: e2 }) => {
                    need_rex!(true, *e2, *ext);
                    e.bytes.push(0x8B);
                    enc_modrm(&mut e, 3, *reg, *r2);
                }
                (Op::R { w: 2, reg, ext }, Op::M { .. }) => {
                    let (m_, rm) = enc_mem(&mut e, b)?;
                    let _ = m_;
                    e.bytes.clear();
                    need_rex!(true, *ext, rm_ext_of(b));
                    e.bytes.push(0x8B);
                    if let Op::M { rip, .. } = b {
                        if rip.is_some() {
                            enc_modrm(&mut e, 0, *reg, 5);
                            let t = syms.get(rip.as_ref().unwrap()).copied().unwrap_or(0);
                            let rel = (t as i64) - (addr as i64 + e.bytes.len() as i64 + 4);
                            enc_imm(&mut e, rel, 4);
                        } else {
                            enc_mem_body(&mut e, b, *reg)?;
                        }
                    } else {
                        enc_mem_body(&mut e, b, *reg)?;
                    }
                }
                (Op::R { w: 1, reg, ext }, Op::M { .. }) => {
                    e.bytes.clear();
                    need_rex!(false, *ext, rm_ext_of(b));
                    e.bytes.push(0x8B);
                    if let Op::M { rip, .. } = b {
                        if rip.is_some() {
                            enc_modrm(&mut e, 0, *reg, 5);
                            let t = syms.get(rip.as_ref().unwrap()).copied().unwrap_or(0);
                            let rel = (t as i64) - (addr as i64 + e.bytes.len() as i64 + 4);
                            enc_imm(&mut e, rel, 4);
                        } else {
                            enc_mem_body(&mut e, b, *reg)?;
                        }
                    } else {
                        enc_mem_body(&mut e, b, *reg)?;
                    }
                }
                (Op::M { .. }, Op::R { w: 2, reg, ext }) => {
                    e.bytes.clear();
                    need_rex!(true, *ext, rm_ext_of(a));
                    e.bytes.push(0x89);
                    if let Op::M { rip, .. } = a {
                        if rip.is_some() {
                            enc_modrm(&mut e, 0, *reg, 5);
                            let t = syms.get(rip.as_ref().unwrap()).copied().unwrap_or(0);
                            let rel = (t as i64) - (addr as i64 + e.bytes.len() as i64 + 4);
                            enc_imm(&mut e, rel, 4);
                        } else {
                            enc_mem_body(&mut e, a, *reg)?;
                        }
                    } else {
                        enc_mem_body(&mut e, a, *reg)?;
                    }
                }
                (Op::M { .. }, Op::R { w: 1, reg, ext }) => {
                    e.bytes.clear();
                    need_rex!(false, *ext, rm_ext_of(a));
                    e.bytes.push(0x89);
                    if let Op::M { rip, .. } = a {
                        if rip.is_some() {
                            enc_modrm(&mut e, 0, *reg, 5);
                            let t = syms.get(rip.as_ref().unwrap()).copied().unwrap_or(0);
                            let rel = (t as i64) - (addr as i64 + e.bytes.len() as i64 + 4);
                            enc_imm(&mut e, rel, 4);
                        } else {
                            enc_mem_body(&mut e, a, *reg)?;
                        }
                    } else {
                        enc_mem_body(&mut e, a, *reg)?;
                    }
                }
                (Op::M { .. }, Op::R { w: 0, reg, ext }) => {
                    e.bytes.clear();
                    need_rex!(false, *ext, rm_ext_of(a));
                    e.bytes.push(0x88);
                    enc_mem_body(&mut e, a, *reg)?;
                }
                _ => return Err(format!("gvtcierk: unsupported mov: {}", text)),
            }
        }
        "movsxd" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::R { w: 2, reg, ext }, Op::R { w: 1, reg: r2, ext: e2 }) = (a, b) {
                need_rex!(true, *e2, *ext);
                e.bytes.push(0x63);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "movzx" => {
            let a = &ops[0];
            let b = &ops[1];
            if let Op::R { w: 1, reg, ext } = a {
                match b {
                    Op::R { w: 0, reg: r2, ext: e2 } => {
                        need_rex!(false, *e2, *ext);
                        e.bytes.extend_from_slice(&[0x0F, 0xB6]);
                        enc_modrm(&mut e, 3, *reg, *r2);
                    }
                    Op::M { .. } => {
                        e.bytes.clear();
                        need_rex!(false, *ext, rm_ext_of(b));
                        e.bytes.extend_from_slice(&[0x0F, 0xB6]);
                        enc_mem_body(&mut e, b, *reg)?;
                    }
                    _ => return Err(format!("gvtcierk: unsupported movzx: {}", text)),
                }
            } else if let Op::R { w: 2, reg, ext } = a {
                match b {
                    Op::R { w: 0, reg: r2, ext: e2 } => {
                        need_rex!(true, *e2, *ext);
                        e.bytes.extend_from_slice(&[0x0F, 0xB6]);
                        enc_modrm(&mut e, 3, *reg, *r2);
                    }
                    Op::M { .. } => {
                        e.bytes.clear();
                        need_rex!(true, *ext, rm_ext_of(b));
                        e.bytes.extend_from_slice(&[0x0F, 0xB6]);
                        enc_mem_body(&mut e, b, *reg)?;
                    }
                    _ => return Err(format!("gvtcierk: unsupported movzx: {}", text)),
                }
            }
        }
        "lea" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::R { w: 2, reg, ext }, Op::M { .. }) = (a, b) {
                e.bytes.clear();
                need_rex!(true, *ext, rm_ext_of(b));
                e.bytes.push(0x8D);
                if let Op::M { rip, .. } = b {
                    if rip.is_some() {
                        enc_modrm(&mut e, 0, *reg, 5);
                        let t = syms.get(rip.as_ref().unwrap()).copied().unwrap_or(0);
                        let rel = (t as i64) - (addr as i64 + e.bytes.len() as i64 + 4);
                        enc_imm(&mut e, rel, 4);
                    } else {
                        enc_mem_body(&mut e, b, *reg)?;
                    }
                } else {
                    enc_mem_body(&mut e, b, *reg)?;
                }
            }
        }
        "add" => {
            let a = &ops[0];
            let b = &ops[1];
            match (a, b) {
                (Op::R { w: 2, reg, ext }, Op::R { w: 2, reg: r2, ext: e2 }) => {
                    need_rex!(true, *e2, *ext);
                    e.bytes.push(0x03);
                    enc_modrm(&mut e, 3, *reg, *r2);
                }
                (Op::R { w: 2, reg: 0, ext: false }, Op::I(v)) => {
                    e.bytes.push(0x48);
                    e.bytes.push(0x05);
                    enc_imm(&mut e, *v, 4);
                }
                (Op::R { w: 2, reg, ext }, Op::I(v)) => {
                    need_rex!(true, false, *ext);
                    e.bytes.push(0x81);
                    enc_modrm(&mut e, 3, 0, *reg);
                    enc_imm(&mut e, *v, 4);
                }
                _ => return Err(format!("gvtcierk: unsupported add: {}", text)),
            }
        }
        "sub" => {
            let a = &ops[0];
            let b = &ops[1];
            match (a, b) {
                (Op::R { w: 2, reg, ext }, Op::R { w: 2, reg: r2, ext: e2 }) => {
                    need_rex!(true, *e2, *ext);
                    e.bytes.push(0x2B);
                    enc_modrm(&mut e, 3, *reg, *r2);
                }
                (Op::R { w: 2, reg, ext }, Op::I(v)) => {
                    if *reg == 4 {
                        e.bytes.push(0x48);
                        e.bytes.push(0x81);
                        enc_modrm(&mut e, 3, 5, *reg);
                        enc_imm(&mut e, *v, 4);
                    } else {
                        need_rex!(true, false, *ext);
                        e.bytes.push(0x81);
                        enc_modrm(&mut e, 3, 5, *reg);
                        enc_imm(&mut e, *v, 4);
                    }
                }
                _ => return Err(format!("gvtcierk: unsupported sub: {}", text)),
            }
        }
        "imul" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::R { w: 2, reg, ext }, Op::R { w: 2, reg: r2, ext: e2 }) = (a, b) {
                need_rex!(true, *e2, *ext);
                e.bytes.extend_from_slice(&[0x0F, 0xAF]);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "idiv" => {
            if let Op::R { w: 2, reg, ext } = &ops[0] {
                need_rex!(true, false, *ext);
                e.bytes.push(0xF7);
                enc_modrm(&mut e, 3, 7, *reg);
            }
        }
        "shl" | "sar" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::R { w: 1, reg, ext }, Op::R { w: 0, reg: 1, ext: false }) = (a, b) {
                need_rex!(false, false, *ext);
                e.bytes.push(0xD3);
                let ext = if mne == "shl" { 4 } else { 7 };
                enc_modrm(&mut e, 3, ext, *reg);
            }
        }
        "and" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::R { w: 1, reg, ext }, Op::R { w: 1, reg: r2, ext: e2 }) = (a, b) {
                need_rex!(false, *e2, *ext);
                e.bytes.push(0x23);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "or" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::R { w: 1, reg, ext }, Op::R { w: 1, reg: r2, ext: e2 }) = (a, b) {
                need_rex!(false, *e2, *ext);
                e.bytes.push(0x0B);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "xor" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::R { w: 1, reg, ext }, Op::R { w: 1, reg: r2, ext: e2 }) = (a, b) {
                need_rex!(false, *e2, *ext);
                e.bytes.push(0x33);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "cmp" => {
            let a = &ops[0];
            let b = &ops[1];
            match (a, b) {
                (Op::R { w: 1, reg, ext }, Op::R { w: 1, reg: r2, ext: e2 }) => {
                    need_rex!(false, *e2, *ext);
                    e.bytes.push(0x3B);
                    enc_modrm(&mut e, 3, *reg, *r2);
                }
                (Op::R { w: 1, reg, ext }, Op::I(v)) => {
                    need_rex!(false, false, *ext);
                    e.bytes.push(0x83);
                    enc_modrm(&mut e, 3, 7, *reg);
                    enc_imm(&mut e, *v, 1);
                }
                _ => return Err(format!("gvtcierk: unsupported cmp: {}", text)),
            }
        }
        s if s.starts_with("set") => {
            if let Op::R { w: 0, reg, ext } = &ops[0] {
                need_rex!(false, false, *ext);
                e.bytes.push(0x0F);
                let cc = match s {
                    "sete" => 0x94,
                    "setne" => 0x95,
                    "setb" => 0x92,
                    "setae" => 0x93,
                    "setbe" => 0x96,
                    "seta" => 0x97,
                    "setl" => 0x9C,
                    "setge" => 0x9D,
                    "setle" => 0x9E,
                    "setg" => 0x9F,
                    _ => return Err(format!("gvtcierk: unsupported set: {}", s)),
                };
                e.bytes.push(cc);
                enc_modrm(&mut e, 3, 0, *reg);
            }
        }
        "call" | "jmp" | "je" => {
            let name = rest.trim().to_string();
            if mne == "je" {
                e.bytes.push(0x0F);
                e.bytes.push(0x84);
            } else {
                let opcode: u8 = if mne == "call" { 0xE8 } else { 0xE9 };
                e.bytes.push(opcode);
            }
            let t = syms.get(&name).copied().unwrap_or(0);
            let rel = if mne == "je" {
                (t as i64) - (addr as i64 + 6)
            } else {
                (t as i64) - (addr as i64 + 5)
            };
            enc_imm(&mut e, rel, 4);
        }
        "movd" => {
            let a = &ops[0];
            let b = &ops[1];
            match (a, b) {
                (Op::X { reg, ext }, Op::R { w: 1, reg: r2, ext: e2 }) => {
                    need_rex!(false, *ext, *e2);
                    e.bytes.extend_from_slice(&[0x66, 0x0F, 0x6E]);
                    enc_modrm(&mut e, 3, *reg, *r2);
                }
                (Op::R { w: 1, reg, ext }, Op::X { reg: r2, ext: e2 }) => {
                    need_rex!(false, *e2, *ext);
                    e.bytes.extend_from_slice(&[0x66, 0x0F, 0x7E]);
                    enc_modrm(&mut e, 3, *reg, *r2);
                }
                _ => return Err(format!("gvtcierk: unsupported movd: {}", text)),
            }
        }
        "movq" => {
            let a = &ops[0];
            let b = &ops[1];
            match (a, b) {
                (Op::X { reg, ext }, Op::R { w: 2, reg: r2, ext: e2 }) => {
                    e.bytes.push(0x66);
                    need_rex!(true, *ext, *e2);
                    e.bytes.extend_from_slice(&[0x0F, 0x6E]);
                    enc_modrm(&mut e, 3, *reg, *r2);
                }
                (Op::R { w: 2, reg, ext }, Op::X { reg: r2, ext: e2 }) => {
                    e.bytes.push(0x66);
                    need_rex!(true, *e2, *ext);
                    e.bytes.extend_from_slice(&[0x0F, 0x7E]);
                    enc_modrm(&mut e, 3, *reg, *r2);
                }
                _ => return Err(format!("gvtcierk: unsupported movq: {}", text)),
            }
        }
        "cvtsi2ss" | "cvtsi2sd" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::X { reg, ext }, Op::R { w: 1, reg: r2, ext: e2 }) = (a, b) {
                need_rex!(false, *ext, *e2);
                if mne == "cvtsi2ss" {
                    e.bytes.push(0xF3);
                } else {
                    e.bytes.push(0xF2);
                }
                e.bytes.extend_from_slice(&[0x0F, 0x2A]);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "cvtss2sd" | "cvtsd2ss" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::X { reg, ext }, Op::X { reg: r2, ext: e2 }) = (a, b) {
                need_rex!(false, *e2, *ext);
                if mne == "cvtss2sd" {
                    e.bytes.push(0xF3);
                } else {
                    e.bytes.push(0xF2);
                }
                e.bytes.extend_from_slice(&[0x0F, 0x5A]);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "cvttss2si" | "cvttsd2si" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::R { w: 2, reg, ext }, Op::X { reg: r2, ext: e2 }) = (a, b) {
                need_rex!(true, *e2, *ext);
                if mne == "cvttss2si" {
                    e.bytes.push(0xF3);
                } else {
                    e.bytes.push(0xF2);
                }
                e.bytes.extend_from_slice(&[0x0F, 0x2C]);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "addss" | "subss" | "mulss" | "divss" | "addsd" | "subsd" | "mulsd" | "divsd" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::X { reg, ext }, Op::X { reg: r2, ext: e2 }) = (a, b) {
                need_rex!(false, *e2, *ext);
                if mne.ends_with("sd") {
                    e.bytes.push(0xF2);
                } else {
                    e.bytes.push(0xF3);
                }
                e.bytes.push(0x0F);
                let op = match mne.as_str() {
                    "addss" | "addsd" => 0x58,
                    "subss" | "subsd" => 0x5C,
                    "mulss" | "mulsd" => 0x59,
                    _ => 0x5E,
                };
                e.bytes.push(op);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        "ucomiss" | "ucomisd" => {
            let a = &ops[0];
            let b = &ops[1];
            if let (Op::X { reg, ext }, Op::X { reg: r2, ext: e2 }) = (a, b) {
                need_rex!(false, *e2, *ext);
                if mne == "ucomisd" {
                    e.bytes.push(0x66);
                }
                e.bytes.extend_from_slice(&[0x0F, 0x2E]);
                enc_modrm(&mut e, 3, *reg, *r2);
            }
        }
        _ => return Err(format!("gvtcierk: unknown instruction: {}", text)),
    }
    Ok(e.bytes)
}

fn enc_mem_body(e: &mut Enc, m: &Op, reg: u8) -> Result<(), String> {
    if let Op::M { base, index, scale, disp, .. } = m {
        let base = base.map(|(b, c)| (b, c));
        let index = index.map(|(b, c)| (b, c));
        let scale = *scale;
        let disp = *disp;
        match (base, index) {
            (Some((_be, bc)), Some((_ie, ic))) => {
                let (mod_, db) = disp_size(disp);
                enc_modrm(e, mod_, reg, 4);
                enc_sib(e, scale, ic, bc);
                enc_disp(e, disp, db);
            }
            (Some((_be, bc)), None) => {
                if bc == 4 {
                    let (mod_, db) = disp_size(disp);
                    enc_modrm(e, mod_, reg, 4);
                    enc_sib(e, 0, 4, bc);
                    enc_disp(e, disp, db);
                } else {
                    let (mod_, db) = disp_size(disp);
                    enc_modrm(e, mod_, reg, bc);
                    enc_disp(e, disp, db);
                }
            }
            (None, Some((_ie, ic))) => {
                let (mod_, _db) = disp_size(disp);
                enc_modrm(e, mod_, reg, 4);
                enc_sib(e, scale, ic, 5);
                enc_imm(e, disp as i64, 4);
            }
            (None, None) => return Err("bad mem body".into()),
        }
    }
    Ok(())
}

fn disp_size(disp: i32) -> (u8, usize) {
    if disp == 0 {
        (0, 0)
    } else if (disp as i8) as i32 == disp {
        (1, 1)
    } else {
        (2, 4)
    }
}

fn enc_disp(e: &mut Enc, disp: i32, n: usize) {
    if n == 1 {
        enc_imm(e, disp as i64, 1);
    } else if n == 4 {
        enc_imm(e, disp as i64, 4);
    }
}

fn rm_ext_of(m: &Op) -> bool {
    if let Op::M { base, index, .. } = m {
        if let Some((e, _)) = index {
            return *e;
        }
        if let Some((e, _)) = base {
            return *e;
        }
    }
    false
}

fn encode_text(items: &[(Sec, Item)], syms: &HashMap<String, usize>) -> Vec<u8> {
    let mut text: Vec<u8> = Vec::new();
    let mut addr = 0usize;
    for (s, it) in items {
        if *s != Sec::Text {
            continue;
        }
        if let Item::Instr(t) = it {
            let bytes = encode(t, addr, syms).unwrap_or_default();
            text.extend_from_slice(&bytes);
            addr += bytes.len();
        }
    }
    text
}

fn assemble(items: &[(Sec, Item)]) -> (Vec<u8>, Vec<u8>, usize, HashMap<String, usize>) {
    let mut syms: HashMap<String, usize> = HashMap::new();
    let mut addr = 0usize;
    for (s, it) in items {
        if *s != Sec::Text {
            continue;
        }
        match it {
            Item::Label(n) => {
                syms.insert(n.clone(), addr);
            }
            Item::Instr(t) => {
                let bytes = encode(t, addr, &HashMap::new()).unwrap_or_default();
                addr += bytes.len();
            }
            _ => {}
        }
    }
    let text_size = addr;

    let mut data_size = 0usize;
    let mut bss_size = 0usize;
    for (s, it) in items {
        match (s, it) {
            (Sec::Data, Item::Data(b)) => data_size += b.len(),
            (Sec::Data, Item::Zero(n)) => data_size += n,
            (Sec::Bss, Item::Zero(n)) => bss_size += n,
            _ => {}
        }
    }
    let base = |sec: Sec, off: usize| match sec {
        Sec::Text => off,
        Sec::Data => text_size + off,
        Sec::Bss => text_size + data_size + off,
    };
    let mut syms2: HashMap<String, usize> = HashMap::new();
    let mut offs = [0usize; 3];
    for (s, it) in items {
        let idx = match s {
            Sec::Text => 0,
            Sec::Data => 1,
            Sec::Bss => 2,
        };
        match it {
            Item::Label(n) => {
                syms2.insert(n.clone(), base(*s, offs[idx]));
            }
            Item::Instr(t) => {
                if *s == Sec::Text {
                    let bytes = encode(t, offs[idx], &HashMap::new()).unwrap_or_default();
                    offs[idx] += bytes.len();
                }
            }
            Item::Data(b) => offs[idx] += b.len(),
            Item::Zero(n) => offs[idx] += n,
        }
    }

    let mut data: Vec<u8> = Vec::new();
    for (s, it) in items {
        match it {
            Item::Data(b) => {
                if *s == Sec::Data {
                    data.extend_from_slice(b);
                }
            }
            Item::Zero(n) => {
                if *s == Sec::Data {
                    data.extend(std::iter::repeat(0u8).take(*n));
                }
            }
            _ => {}
        }
    }
    let text = encode_text(items, &syms2);
    (text, data, bss_size, syms2)
}

const IMAGE_BASE: u64 = 0x140000000;
const SEC_ALIGN: u32 = 0x1000;
const FILE_ALIGN: u32 = 0x200;
const IMPORTS: [&str; 6] = [
    "ExitProcess",
    "GetStdHandle",
    "CreateFileA",
    "ReadFile",
    "WriteFile",
    "CloseHandle",
];

fn align_up(v: usize, a: u32) -> usize {
    (v + a as usize - 1) & !(a as usize - 1)
}

fn build_import_region(import_off: usize) -> (Vec<u8>, Vec<usize>) {
    let base = 0x1000usize + import_off;
    let mut region: Vec<u8> = Vec::new();
    let idt_off = region.len();
    region.extend_from_slice(&[0u8; 40]);
    let int_off = align_up(region.len(), 8);
    while region.len() < int_off {
        region.push(0);
    }
    let iat_off = int_off + (IMPORTS.len() + 1) * 8;
    let name_base = align_up(iat_off + (IMPORTS.len() + 1) * 8, 8);
    let mut name_rvas: Vec<u32> = Vec::new();
    let mut name_bytes: Vec<u8> = Vec::new();
    for f in IMPORTS.iter() {
        let off = name_bytes.len();
        name_rvas.push((base + name_base + off) as u32);
        name_bytes.extend_from_slice(&[0u8, 0]);
        name_bytes.extend_from_slice(f.as_bytes());
        name_bytes.push(0);
    }
    let dll_off = name_base + name_bytes.len();
    let dll_rva = (base + dll_off) as u32;
    region.resize(name_base + name_bytes.len() + 13, 0);
    for (i, nr) in name_rvas.iter().enumerate() {
        let p = int_off + i * 8;
        region[p..p + 8].copy_from_slice(&(*nr as u64).to_le_bytes());
    }
    let mut iat_offsets: Vec<usize> = Vec::new();
    for (i, nr) in name_rvas.iter().enumerate() {
        let p = iat_off + i * 8;
        region[p..p + 8].copy_from_slice(&(*nr as u64).to_le_bytes());
        iat_offsets.push(p);
    }
    region[name_base..name_base + name_bytes.len()].copy_from_slice(&name_bytes);
    let dll = b"kernel32.dll\0";
    region[dll_off..dll_off + dll.len()].copy_from_slice(dll);
    let p = idt_off;
    region[p..p + 4].copy_from_slice(&((base + int_off) as u32).to_le_bytes());
    region[p + 4..p + 8].copy_from_slice(&0u32.to_le_bytes());
    region[p + 8..p + 12].copy_from_slice(&0u32.to_le_bytes());
    region[p + 12..p + 16].copy_from_slice(&dll_rva.to_le_bytes());
    region[p + 16..p + 20].copy_from_slice(&((base + iat_off) as u32).to_le_bytes());
    (region, iat_offsets)
}

fn build_start(main_rva: u64, iat0_rva: u64, start_rva: u64) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);
    let call_addr = start_rva + 4;
    let rel = (main_rva as i64) - (call_addr as i64 + 5);
    b.push(0xE8);
    b.extend_from_slice(&(rel as i32).to_le_bytes());
    b.extend_from_slice(&[0x8B, 0xC8]);
    let call_rip_addr = start_rva + 4 + 5 + 2;
    let rel2 = (iat0_rva as i64) - (call_rip_addr as i64 + 6);
    b.extend_from_slice(&[0xFF, 0x15]);
    b.extend_from_slice(&(rel2 as i32).to_le_bytes());
    b
}

fn build_runtime(iats: &HashMap<String, usize>, base_rva: usize) -> (Vec<u8>, HashMap<String, usize>) {
    let mut b: Vec<u8> = Vec::new();
    let dayin_rva = base_rva;
    b.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x83, 0xEC, 0x70]);
    b.extend_from_slice(&[0x48, 0x89, 0x4D, 0xF8]);
    b.extend_from_slice(&[0x48, 0x89, 0x55, 0xF0]);
    b.extend_from_slice(&[0x83, 0xFA, 0x01]);
    b.extend_from_slice(&[0x0F, 0x85, 0, 0, 0, 0]);
    let jne1 = b.len();
    b.extend_from_slice(&[0x48, 0x8B, 0x45, 0xF8]);
    b.extend_from_slice(&[0x45, 0x31, 0xD2]);
    b.extend_from_slice(&[0x45, 0x31, 0xDB]);
    b.extend_from_slice(&[0x48, 0x85, 0xC0]);
    b.push(0x75);
    let jne2 = b.len();
    b.push(0);
    b.extend_from_slice(&[0xC6, 0x45, 0xB8, 0x30]);
    b.extend_from_slice(&[0x48, 0xC7, 0x45, 0xE8, 1, 0, 0, 0]);
    b.extend_from_slice(&[0xE9, 0, 0, 0, 0]);
    let jmp1 = b.len();
    let nonz = b.len();
    b[jne2] = (nonz as i64 - (jne2 as i64 + 1)) as u8;
    b.extend_from_slice(&[0x48, 0x85, 0xC0]);
    b.push(0x79);
    let jns1 = b.len();
    b.push(0);
    b.extend_from_slice(&[0x41, 0xBB, 1, 0, 0, 0]);
    b.extend_from_slice(&[0x48, 0xF7, 0xD8]);
    let pos = b.len();
    b[jns1] = (pos as i64 - (jns1 as i64 + 1)) as u8;
    let loop_start = b.len();
    b.extend_from_slice(&[0x31, 0xD2]);
    b.extend_from_slice(&[0xB9, 0x0A, 0, 0, 0]);
    b.extend_from_slice(&[0x48, 0xF7, 0xF1]);
    b.extend_from_slice(&[0x48, 0x83, 0xC2, 0x30]);
    b.extend_from_slice(&[0x42, 0x88, 0x54, 0x15, 0xB8]);
    b.extend_from_slice(&[0x49, 0xFF, 0xC2]);
    b.extend_from_slice(&[0x48, 0x85, 0xC0]);
    b.push(0x75);
    let jne3 = b.len();
    b.push(0);
    b.extend_from_slice(&[0x45, 0x85, 0xDB]);
    b.push(0x74);
    let je1 = b.len();
    b.push(0);
    b.extend_from_slice(&[0x42, 0xC6, 0x44, 0x15, 0xB8, 0x2D]);
    b.extend_from_slice(&[0x49, 0xFF, 0xC2]);
    let rev = b.len();
    b[jne3] = (loop_start as i64 - (jne3 as i64 + 1)) as u8;
    b[je1] = (rev as i64 - (je1 as i64 + 1)) as u8;
    b.extend_from_slice(&[0x31, 0xC9]);
    b.extend_from_slice(&[0x4C, 0x89, 0xD2]);
    b.extend_from_slice(&[0x48, 0xFF, 0xCA]);
    let rev_loop = b.len();
    b.extend_from_slice(&[0x48, 0x39, 0xD1]);
    b.push(0x7D);
    let jge1 = b.len();
    b.push(0);
    b.extend_from_slice(&[0x8A, 0x44, 0x0D, 0xB8]);
    b.extend_from_slice(&[0x8A, 0x5C, 0x15, 0xB8]);
    b.extend_from_slice(&[0x88, 0x5C, 0x0D, 0xB8]);
    b.extend_from_slice(&[0x88, 0x44, 0x15, 0xB8]);
    b.extend_from_slice(&[0x48, 0xFF, 0xC1]);
    b.extend_from_slice(&[0x48, 0xFF, 0xCA]);
    b.push(0xEB);
    let jmp2 = b.len();
    b.push(0);
    let rev_end = b.len();
    b[jge1] = (rev_end as i64 - (jge1 as i64 + 1)) as u8;
    b[jmp2] = (rev_loop as i64 - (jmp2 as i64 + 1)) as u8;
    b.extend_from_slice(&[0x4C, 0x89, 0x55, 0xE8]);
    b.extend_from_slice(&[0xE9, 0, 0, 0, 0]);
    let jmp3 = b.len();
    let str_path = b.len();
    b[jne1 - 4..jne1].copy_from_slice(&((str_path as i64 - jne1 as i64) as i32).to_le_bytes());
    b.extend_from_slice(&[0x48, 0x8B, 0x55, 0xF8]);
    b.extend_from_slice(&[0x48, 0x89, 0x55, 0xD0]);
    b.extend_from_slice(&[0x31, 0xC0]);
    let sloop = b.len();
    b.extend_from_slice(&[0x80, 0x3A, 0x00]);
    b.push(0x74);
    let sje = b.len();
    b.push(0);
    b.extend_from_slice(&[0x48, 0xFF, 0xC2]);
    b.extend_from_slice(&[0xFF, 0xC0]);
    b.push(0xEB);
    let sjmp = b.len();
    b.push(0);
    let sloop_end = b.len();
    b[sje] = (sloop_end as i64 - (sje as i64 + 1)) as u8;
    b[sjmp] = (sloop as i64 - (sjmp as i64 + 1)) as u8;
    b.extend_from_slice(&[0x89, 0x45, 0xE8]);
    b.extend_from_slice(&[0x48, 0x8B, 0x55, 0xD0]);
    b.extend_from_slice(&[0x48, 0x89, 0x55, 0xE0]);
    b.push(0xEB);
    let jmp4 = b.len();
    b.push(0);
    let int_ptr = b.len();
    b[jmp1 - 4..jmp1].copy_from_slice(&((int_ptr as i64 - jmp1 as i64) as i32).to_le_bytes());
    b[jmp3 - 4..jmp3].copy_from_slice(&((int_ptr as i64 - jmp3 as i64) as i32).to_le_bytes());
    b.extend_from_slice(&[0x48, 0x8D, 0x55, 0xB8]);
    b.extend_from_slice(&[0x48, 0x89, 0x55, 0xE0]);
    let out = b.len();
    b[jmp4] = (out as i64 - (jmp4 as i64 + 1)) as u8;
    b.extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]);
    let gsh = iats.get("GetStdHandle").copied().unwrap_or(0);
    let a = base_rva + b.len();
    b.extend_from_slice(&[0xFF, 0x15]);
    b.extend_from_slice(&((gsh as i64 - (a as i64 + 6)) as i32).to_le_bytes());
    b.extend_from_slice(&[0x48, 0x89, 0xC1]);
    b.extend_from_slice(&[0x48, 0x8B, 0x55, 0xE0]);
    b.extend_from_slice(&[0x44, 0x8B, 0x45, 0xE8]);
    b.extend_from_slice(&[0x4C, 0x8D, 0x4D, 0xDC]);
    b.extend_from_slice(&[0xC7, 0x45, 0xDC, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0, 0, 0, 0]);
    let wca = iats.get("WriteFile").copied().unwrap_or(0);
    let a2 = base_rva + b.len();
    b.extend_from_slice(&[0xFF, 0x15]);
    b.extend_from_slice(&((wca as i64 - (a2 as i64 + 6)) as i32).to_le_bytes());
    b.extend_from_slice(&[0x48, 0x83, 0xC4, 0x70]);
    b.extend_from_slice(&[0x5D, 0xC3]);
    let fopen_rva = base_rva + b.len();
    b.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x83, 0xEC, 0x40]);
    b.extend_from_slice(&[0x48, 0x89, 0x4D, 0xF8]);
    b.extend_from_slice(&[0x48, 0x89, 0x55, 0xF0]);
    b.extend_from_slice(&[0x48, 0x8B, 0x45, 0xF0]);
    b.extend_from_slice(&[0x0F, 0xB6, 0x00]);
    b.extend_from_slice(&[0x3C, 0x72]);
    b.push(0x74);
    let je1 = b.len();
    b.push(0);
    b.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x40]);
    b.extend_from_slice(&[0x45, 0x31, 0xC0]);
    b.extend_from_slice(&[0x45, 0x31, 0xC9]);
    b.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 2, 0, 0, 0]);
    b.push(0xEB);
    let jmp1 = b.len();
    b.push(0);
    let read_mode = b.len();
    b.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x80]);
    b.extend_from_slice(&[0x45, 0x31, 0xC0]);
    b.extend_from_slice(&[0x45, 0x31, 0xC9]);
    b.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 3, 0, 0, 0]);
    let args_ready = b.len();
    b[je1] = (read_mode as i64 - (je1 as i64 + 1)) as u8;
    b[jmp1] = (args_ready as i64 - (jmp1 as i64 + 1)) as u8;
    b.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x28, 0x80, 0, 0, 0]);
    b.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x30, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xF8]);
    let cfa = iats.get("CreateFileA").copied().unwrap_or(0);
    let a = base_rva + b.len();
    b.extend_from_slice(&[0xFF, 0x15]);
    b.extend_from_slice(&((cfa as i64 - (a as i64 + 6)) as i32).to_le_bytes());
    b.extend_from_slice(&[0x48, 0x83, 0xC4, 0x40]);
    b.extend_from_slice(&[0x5D, 0xC3]);
    let fread_rva = base_rva + b.len();
    b.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x83, 0xEC, 0x50]);
    b.extend_from_slice(&[0x48, 0x89, 0x4D, 0xF8]);
    b.extend_from_slice(&[0x48, 0x89, 0x55, 0xF0]);
    b.extend_from_slice(&[0x4C, 0x89, 0x45, 0xE8]);
    b.extend_from_slice(&[0x4C, 0x89, 0x4D, 0xE0]);
    b.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xE0]);
    b.extend_from_slice(&[0x48, 0x8B, 0x55, 0xF8]);
    b.extend_from_slice(&[0x48, 0x8B, 0x45, 0xF0]);
    b.extend_from_slice(&[0x48, 0x0F, 0xAF, 0x45, 0xE8]);
    b.extend_from_slice(&[0x49, 0x89, 0xC0]);
    b.extend_from_slice(&[0x4C, 0x8D, 0x4D, 0xD8]);
    b.extend_from_slice(&[0x48, 0xC7, 0x45, 0xD8, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0, 0, 0, 0]);
    let rf = iats.get("ReadFile").copied().unwrap_or(0);
    let a = base_rva + b.len();
    b.extend_from_slice(&[0xFF, 0x15]);
    b.extend_from_slice(&((rf as i64 - (a as i64 + 6)) as i32).to_le_bytes());
    b.extend_from_slice(&[0x48, 0x8B, 0x45, 0xD8]);
    b.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xF0]);
    b.extend_from_slice(&[0x31, 0xD2]);
    b.extend_from_slice(&[0x48, 0xF7, 0xF1]);
    b.extend_from_slice(&[0x48, 0x83, 0xC4, 0x50]);
    b.extend_from_slice(&[0x5D, 0xC3]);
    let fwrite_rva = base_rva + b.len();
    b.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x83, 0xEC, 0x50]);
    b.extend_from_slice(&[0x48, 0x89, 0x4D, 0xF8]);
    b.extend_from_slice(&[0x48, 0x89, 0x55, 0xF0]);
    b.extend_from_slice(&[0x4C, 0x89, 0x45, 0xE8]);
    b.extend_from_slice(&[0x4C, 0x89, 0x4D, 0xE0]);
    b.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xE0]);
    b.extend_from_slice(&[0x48, 0x8B, 0x55, 0xF8]);
    b.extend_from_slice(&[0x48, 0x8B, 0x45, 0xF0]);
    b.extend_from_slice(&[0x48, 0x0F, 0xAF, 0x45, 0xE8]);
    b.extend_from_slice(&[0x49, 0x89, 0xC0]);
    b.extend_from_slice(&[0x4C, 0x8D, 0x4D, 0xD8]);
    b.extend_from_slice(&[0x48, 0xC7, 0x45, 0xD8, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0, 0, 0, 0]);
    let wf = iats.get("WriteFile").copied().unwrap_or(0);
    let a = base_rva + b.len();
    b.extend_from_slice(&[0xFF, 0x15]);
    b.extend_from_slice(&((wf as i64 - (a as i64 + 6)) as i32).to_le_bytes());
    b.extend_from_slice(&[0x48, 0x8B, 0x45, 0xD8]);
    b.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xF0]);
    b.extend_from_slice(&[0x31, 0xD2]);
    b.extend_from_slice(&[0x48, 0xF7, 0xF1]);
    b.extend_from_slice(&[0x48, 0x83, 0xC4, 0x50]);
    b.extend_from_slice(&[0x5D, 0xC3]);
    let fclose_rva = base_rva + b.len();
    b.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x83, 0xEC, 0x20]);
    let ch = iats.get("CloseHandle").copied().unwrap_or(0);
    let a = base_rva + b.len();
    b.extend_from_slice(&[0xFF, 0x15]);
    b.extend_from_slice(&((ch as i64 - (a as i64 + 6)) as i32).to_le_bytes());
    b.extend_from_slice(&[0x48, 0x83, 0xC4, 0x20]);
    b.extend_from_slice(&[0x5D, 0xC3]);
    let mut syms = HashMap::new();
    syms.insert("DaYin".to_string(), dayin_rva);
    syms.insert("fopen".to_string(), fopen_rva);
    syms.insert("fread".to_string(), fread_rva);
    syms.insert("fwrite".to_string(), fwrite_rva);
    syms.insert("fclose".to_string(), fclose_rva);
    (b, syms)
}

fn build_pe(
    text: &[u8],
    data: &[u8],
    bss: usize,
    start: &[u8],
    runtime: &[u8],
    import: &[u8],
    entry_rva: u32,
    import_rva: u32,
    import_size: u32,
) -> Vec<u8> {
    let mut sec_data = Vec::new();
    sec_data.extend_from_slice(text);
    sec_data.extend_from_slice(data);
    sec_data.extend_from_slice(start);
    sec_data.extend_from_slice(runtime);
    sec_data.extend_from_slice(import);
    let sec_virt = sec_data.len() + bss;
    let size_of_image = align_up(0x1000 + sec_virt, SEC_ALIGN) as u32;
    let size_of_headers = align_up(0x200, FILE_ALIGN) as u32;
    let raw_size = align_up(sec_data.len(), FILE_ALIGN) as u32;

    let mut f = Vec::new();
    f.extend_from_slice(b"MZ");
    f.extend_from_slice(&[0u8; 58]);
    f.extend_from_slice(&0x80u32.to_le_bytes());
    f.resize(0x80, 0);
    f.extend_from_slice(b"PE\0\0");
    f.extend_from_slice(&0x8664u16.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&240u16.to_le_bytes());
    f.extend_from_slice(&0x22u16.to_le_bytes());
    let opt_off = f.len();
    f.extend_from_slice(&0x20Bu16.to_le_bytes());
    f.push(0);
    f.push(0);
    f.extend_from_slice(&(text.len() as u32).to_le_bytes());
    f.extend_from_slice(&((data.len() + import.len()) as u32).to_le_bytes());
    f.extend_from_slice(&(bss as u32).to_le_bytes());
    f.extend_from_slice(&entry_rva.to_le_bytes());
    f.extend_from_slice(&0x1000u32.to_le_bytes());
    f.extend_from_slice(&IMAGE_BASE.to_le_bytes());
    f.extend_from_slice(&SEC_ALIGN.to_le_bytes());
    f.extend_from_slice(&FILE_ALIGN.to_le_bytes());
    f.extend_from_slice(&6u16.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.extend_from_slice(&6u16.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&size_of_image.to_le_bytes());
    f.extend_from_slice(&size_of_headers.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&3u16.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.extend_from_slice(&0u64.to_le_bytes());
    f.extend_from_slice(&0x40000000u64.to_le_bytes());
    f.extend_from_slice(&0x1000000u64.to_le_bytes());
    f.extend_from_slice(&0x1000u64.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&16u32.to_le_bytes());
    for i in 0..16 {
        if i == 1 {
            f.extend_from_slice(&import_rva.to_le_bytes());
            f.extend_from_slice(&import_size.to_le_bytes());
        } else {
            f.extend_from_slice(&[0u8; 8]);
        }
    }
    debug_assert_eq!(f.len() - opt_off, 240);
    f.extend_from_slice(b".text\0\0\0");
    f.extend_from_slice(&(sec_virt as u32).to_le_bytes());
    f.extend_from_slice(&0x1000u32.to_le_bytes());
    f.extend_from_slice(&raw_size.to_le_bytes());
    f.extend_from_slice(&size_of_headers.to_le_bytes());
    f.extend_from_slice(&[0u8; 12]);
    f.extend_from_slice(&0xE0000020u32.to_le_bytes());
    f.resize(size_of_headers as usize, 0);
    f.extend_from_slice(&sec_data);
    f.resize(size_of_headers as usize + raw_size as usize, 0);
    f
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: gvtcierk <file.s>");
        std::process::exit(1);
    }
    let src = fs::read_to_string(&args[1]).expect("read asm");
    let items = parse(&src);
    let (text0, data, bss, mut syms) = assemble(&items);
    let start_len = 17usize;
    let (runtime_probe, _) = build_runtime(&HashMap::new(), 0);
    let runtime_len = runtime_probe.len();
    let text_data_len = text0.len() + data.len();
    let start_rva = (0x1000 + text_data_len) as u64;
    let runtime_rva = 0x1000 + text_data_len + start_len;
    let import_off = text_data_len + start_len + runtime_len;
    let (import, iat_offs) = build_import_region(import_off);
    let mut iats: HashMap<String, usize> = HashMap::new();
    for (i, name) in IMPORTS.iter().enumerate() {
        iats.insert(name.to_string(), 0x1000 + import_off + iat_offs[i]);
    }
    let main_rva = 0x1000u64 + *syms.get("main").unwrap_or(&0) as u64;
    let start = build_start(main_rva, iats["ExitProcess"] as u64, start_rva);
    let (runtime, runtime_syms) = build_runtime(&iats, runtime_rva);
    for (n, rva) in runtime_syms {
        syms.insert(n, rva - 0x1000);
    }
    let text = encode_text(&items, &syms);
    let import_rva = (0x1000 + import_off) as u32;
    let exe = build_pe(
        &text,
        &data,
        bss,
        &start,
        &runtime,
        &import,
        start_rva as u32,
        import_rva,
        import.len() as u32,
    );
    let out = args[1].replace(".s", ".exe");
    fs::write(&out, &exe).expect("write exe");
    println!(
        "gvtcierk: {} -> {} ({} bytes, text={} data={} bss={})",
        args[1],
        out,
        exe.len(),
        text.len(),
        data.len(),
        bss
    );
}
