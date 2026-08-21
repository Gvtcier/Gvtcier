const COM1: u16 = 0x3F8;
const COM2: u16 = 0x2F8;

pub fn init() {
    init_port(COM1);
    init_port(COM2);
}

fn init_port(port: u16) {
    unsafe {
        write_reg(port + 1, 0x00);
        write_reg(port + 3, 0x80);
        write_reg(port + 0, 0x03);
        write_reg(port + 1, 0x00);
        write_reg(port + 3, 0x03);
        write_reg(port + 2, 0xC7);
        write_reg(port + 4, 0x0B);
    }
}

pub fn write_str(s: &str) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            write_byte(crate::io::Gv2280::ascii_to_gv(b));
            i += 1;
        } else if b >= 0xE0 && i + 2 < bytes.len() {
            let ch = [b, bytes[i + 1], bytes[i + 2]];
            if let Some(code) = crate::io::Gv2280::hanzi_code(&ch) {
                write_byte(0x80 + (code >> 8) as u8);
                write_byte(code as u8);
            }
            i += 3;
        } else {
            i += 1;
        }
    }
}

pub fn print_str(s: &str) {
    write_str(s);
}

pub fn print_hex(v: u64) {
    let mut buf = [0u8; 18];
    buf[0] = crate::io::Gv2280::ascii_to_gv(b'0');
    buf[1] = crate::io::Gv2280::ascii_to_gv(b'x');
    let mut o = 2;
    for i in (0..16).rev() {
        let d = ((v >> (i * 4)) & 0xF) as u8;
        buf[o] = crate::io::Gv2280::ascii_to_gv(if d < 10 { b'0' + d } else { b'a' + d - 10 });
        o += 1;
    }
    write_bytes(&buf);
}

pub fn print_hex2(b: u8) {
    let mut buf = [0u8; 2];
    buf[0] = crate::io::Gv2280::ascii_to_gv(if b >> 4 < 10 { b'0' + (b >> 4) } else { b'a' + (b >> 4) - 10 });
    buf[1] = crate::io::Gv2280::ascii_to_gv(if b & 0xF < 10 { b'0' + (b & 0xF) } else { b'a' + (b & 0xF) - 10 });
    write_bytes(&buf);
}

pub fn read_ready() -> bool {
    unsafe {
        let st: u8;
        core::arch::asm!("in al, dx", out("al") st, in("dx") COM1 + 5, options(nomem, nostack));
        st & 1 != 0
    }
}

pub fn read_byte() -> u8 {
    unsafe {
        let v: u8;
        core::arch::asm!("in al, dx", out("al") v, in("dx") COM1, options(nomem, nostack));
        v
    }
}

pub fn read_str(out: &mut [u8]) -> usize {
    let mut n = 0;
    while n < out.len() {
        if !read_ready() {
            break;
        }
        let b = read_byte();
        if b == b'\r' || b == b'\n' {
            break;
        }
        out[n] = b;
        n += 1;
    }
    n
}

pub fn write_bytes(b: &[u8]) {
    for x in b {
        write_byte(*x);
    }
}

pub fn write_byte(b: u8) {
    write_byte_port(COM1, b);
}

pub fn write_byte_port(port: u16, b: u8) {
    let mut spins = 0u32;
    loop {
        let lsr = unsafe { read_reg(port + 5) };
        if lsr & 0x20 != 0 {
            break;
        }
        spins += 1;
        if spins > 1000000 {
            break;
        }
    }
    unsafe { write_reg(port, b) };
}

unsafe fn write_reg(port: u16, v: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack));
}

unsafe fn read_reg(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack));
    v
}
