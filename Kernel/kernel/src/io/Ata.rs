unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack));
    v
}

unsafe fn outb(port: u16, v: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack));
}

unsafe fn inw(port: u16) -> u16 {
    let v: u16;
    core::arch::asm!("in ax, dx", out("ax") v, in("dx") port, options(nomem, nostack));
    v
}

pub fn read_sector(lba: u32, buf: &mut [u8; 512]) -> u32 {
    unsafe {
        let st0 = inb(0x1F7);
        let mut t0 = 0;
        while inb(0x1F7) & 0x80 != 0 {
            t0 += 1;
            if t0 > 1000000 {
                crate::println!("ata: bsy timeout lba={} st0={:#x}", lba, st0);
                return 1;
            }
        }
        outb(0x1F6, 0xE0 | ((lba >> 24) & 0x0F) as u8);
        outb(0x1F2, 1);
        outb(0x1F3, lba as u8);
        outb(0x1F4, (lba >> 8) as u8);
        outb(0x1F5, (lba >> 16) as u8);
        outb(0x1F7, 0x20);
        let mut t = 0;
        while t < 100000 {
            let st = inb(0x1F7);
            if st & 0x80 == 0 {
                break;
            }
            t += 1;
        }
        if t >= 100000 {
            crate::println!("ata: cmd bsy timeout lba={}", lba);
            return 1;
        }
        let st_bsy = inb(0x1F7);
        let mut td = 0;
        while inb(0x1F7) & 0x08 == 0 {
            td += 1;
            if td > 1000000 {
                crate::println!("ata: drq timeout lba={} st={:#x}", lba, st_bsy);
                return 1;
            }
        }
        let st_drq = inb(0x1F7);
        for i in 0..256 {
            let w = inw(0x1F0);
            buf[i * 2] = w as u8;
            buf[i * 2 + 1] = (w >> 8) as u8;
        }
        crate::println!(
            "ata: lba={} st0={:#x} st_bsy={:#x} st_drq={:#x} t={} td={} b0={:#x} b1={:#x}",
            lba,
            st0,
            st_bsy,
            st_drq,
            t,
            td,
            buf[0],
            buf[1]
        );
    }
    0
}

pub fn read(lba: u32, count: u32, buf: &mut [u8]) -> u32 {
    for i in 0..count {
        let mut sector = [0u8; 512];
        read_sector(lba + i, &mut sector);
        let dst = (i as usize) * 512;
        buf[dst..dst + 512].copy_from_slice(&sector);
    }
    0
}
