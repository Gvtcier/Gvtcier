use crate::intr::Apic;

unsafe fn cmos_read(reg: u8) -> u8 {
    core::arch::asm!("out dx, al", in("dx") 0x70u16, in("al") reg, options(nomem, nostack));
    let v: u8;
    core::arch::asm!("in al, dx", out("al") v, in("dx") 0x71u16, options(nomem, nostack));
    v
}

fn bcd(v: u8) -> u8 {
    (v >> 4) * 10 + (v & 0x0F)
}

pub fn now() -> (u8, u8, u8) {
    let sec = unsafe { cmos_read(0x00) };
    let min = unsafe { cmos_read(0x02) };
    let hour = unsafe { cmos_read(0x04) };
    (bcd(hour), bcd(min), bcd(sec))
}

pub fn date() -> (u8, u8, u16) {
    let day = bcd(unsafe { cmos_read(0x07) });
    let mon = bcd(unsafe { cmos_read(0x08) });
    let year = bcd(unsafe { cmos_read(0x09) }) as u16;
    let cent = bcd(unsafe { cmos_read(0x32) }) as u16;
    (day, mon, cent * 100 + year)
}

pub fn weekday() -> u8 {
    bcd(unsafe { cmos_read(0x06) })
}

pub fn uptime_sec() -> u64 {
    crate::intr::Apic::tick() / 100
}

pub fn format(buf: &mut [u8]) -> usize {
    let (day, mon, year) = date();
    let (hh, mm, ss) = now();
    let mut o = 0;
    let w = |b: &[u8], buf: &mut [u8], o: &mut usize| {
        let mut i = 0;
        while i < b.len() {
            if *o < buf.len() {
                buf[*o] = b[i];
                *o += 1;
            }
            i += 1;
        }
    };
    let mut t = [0u8; 4];
    t[0] = b'0' + (year / 1000) as u8;
    t[1] = b'0' + ((year / 100) % 10) as u8;
    t[2] = b'0' + ((year / 10) % 10) as u8;
    t[3] = b'0' + (year % 10) as u8;
    w(&t, buf, &mut o);
    w(b"-", buf, &mut o);
    let m = [b'0' + mon / 10, b'0' + mon % 10];
    w(&m, buf, &mut o);
    w(b"-", buf, &mut o);
    let d = [b'0' + day / 10, b'0' + day % 10];
    w(&d, buf, &mut o);
    w(b" ", buf, &mut o);
    let h = [b'0' + hh / 10, b'0' + hh % 10];
    w(&h, buf, &mut o);
    w(b":", buf, &mut o);
    let mi = [b'0' + mm / 10, b'0' + mm % 10];
    w(&mi, buf, &mut o);
    w(b":", buf, &mut o);
    let s = [b'0' + ss / 10, b'0' + ss % 10];
    w(&s, buf, &mut o);
    o
}
