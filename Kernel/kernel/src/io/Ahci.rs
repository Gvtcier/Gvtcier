const PCI_ADDR: u16 = 0xCF8;
const PCI_DATA: u16 = 0xCFC;

static mut ABAR: usize = 0;
static mut PORT_BASE: usize = 0x100;
static mut CMD_BUF: [u8; 512] = [0; 512];

unsafe fn outl(port: u16, v: u32) {
    core::arch::asm!("out dx, eax", in("dx") port, in("eax") v, options(nomem, nostack));
}

unsafe fn inl(port: u16) -> u32 {
    let v: u32;
    core::arch::asm!("in eax, dx", out("eax") v, in("dx") port, options(nomem, nostack));
    v
}

fn pci_read(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    unsafe {
        outl(
            PCI_ADDR,
            0x80000000
                | ((bus as u32) << 16)
                | ((dev as u32) << 11)
                | ((func as u32) << 8)
                | ((off as u32) & 0xFC),
        );
        inl(PCI_DATA)
    }
}

pub fn init() -> u32 {
    unsafe {
        for dev in 0..32 {
            for func in 0..8 {
                let class = (pci_read(0, dev, func, 0x08) >> 16) & 0xFFFF;
                if class == 0x0106 {
                    let bar5 = pci_read(0, dev, func, 0x24);
                    ABAR = (bar5 & 0xFFFFFFF0) as usize;
                    let ghc = (ABAR + 0x04) as *mut u32;
                    ghc.write_volatile(ghc.read_volatile() | (1 << 31));
                    const CLB_FIXED: usize = 0x78000;
                    const FB_FIXED: usize = 0x79000;
                    let clb_phys = CLB_FIXED as u32;
                    let fb_phys = FB_FIXED as u32;
                    let clb_virt = CLB_FIXED as *mut u8;
                    for i in 0..4096 {
                        clb_virt.add(i).write_volatile(0);
                    }
                    let fb_virt = FB_FIXED as *mut u8;
                    for i in 0..4096 {
                        fb_virt.add(i).write_volatile(0);
                    }
                    let pcmd = (ABAR + 0x118) as *mut u32;
                    pcmd.write_volatile(0);
                    ((ABAR + 0x100) as *mut u32).write_volatile(clb_phys);
                    ((ABAR + 0x108) as *mut u32).write_volatile(fb_phys);
                    pcmd.write_volatile(0x11);
                    crate::println!("ahci: ABAR={:#x} clb={:#x} fb={:#x} ready", ABAR, clb_phys, fb_phys);
                    return 0;
                }
            }
        }
    }
    1
}

pub fn read_sector(lba: u64, buf: &mut [u8; 512]) -> u32 {
    unsafe {
        if ABAR == 0 {
            return 1;
        }
        let cl_base = (ABAR + PORT_BASE) as *const u32;
        let mut cl_phys = cl_base.read_volatile() as usize;
        crate::println!("ahci: rd lba={} clb={:#x}", lba, cl_phys);
        if cl_phys & 0x1 != 0 {
            return 1;
        }
        cl_phys &= !0x3FF;
        let cmd = cl_phys as *mut u8;
        (cmd as *mut u32).write_volatile(0x10005);
        (cmd.add(4) as *mut u32).write_volatile(0);
        (cmd.add(8) as *mut u32).write_volatile((cl_phys + 0x80) as u32);
        (cmd.add(12) as *mut u32).write_volatile(0);
        let fis = cmd.add(0x80);
        fis.write_volatile(0x27);
        fis.add(1).write_volatile(0x80);
        fis.add(2).write_volatile(0x25);
        fis.add(3).write_volatile(0);
        fis.add(4).write_volatile(lba as u8);
        fis.add(5).write_volatile((lba >> 8) as u8);
        fis.add(6).write_volatile((lba >> 16) as u8);
        fis.add(7).write_volatile(0xE0);
        fis.add(8).write_volatile((lba >> 24) as u8);
        fis.add(9).write_volatile((lba >> 32) as u8);
        fis.add(10).write_volatile((lba >> 40) as u8);
        fis.add(11).write_volatile(0);
        fis.add(12).write_volatile(1);
        fis.add(13).write_volatile(0);
        fis.add(14).write_volatile(0);
        fis.add(15).write_volatile(0);
        fis.add(16).write_volatile(0);
        fis.add(17).write_volatile(0);
        fis.add(18).write_volatile(0);
        fis.add(19).write_volatile(0);
        let prdt = cmd.add(0x100);
        let data_phys = CMD_BUF.as_ptr() as usize - 0xFFFF800000000000;
        (prdt as *mut u64).write_volatile(data_phys as u64);
        prdt.add(8).write_volatile(0);
        (prdt.add(12) as *mut u32).write_volatile(0x1FF);
        let ci = (ABAR + PORT_BASE + 0x38) as *mut u32;
        ci.write_volatile(1);
        crate::println!("ahci: rd ci set");
        let mut t = 0;
        while ci.read_volatile() & 1 != 0 {
            t += 1;
            if t > 10000000 {
                let ssts = ((ABAR + PORT_BASE + 0x28) as *const u32).read_volatile();
                let tfd = ((ABAR + PORT_BASE + 0x20) as *const u32).read_volatile();
                crate::println!("ahci: timeout lba={} ssts={:#x} tfd={:#x}", lba, ssts, tfd);
                return 1;
            }
            core::hint::spin_loop();
        }
        crate::println!("ahci: rd ok lba={}", lba);
        let is = (ABAR + PORT_BASE + 0x10) as *mut u32;
        is.write_volatile(is.read_volatile());
        core::ptr::copy_nonoverlapping(CMD_BUF.as_ptr(), buf.as_mut_ptr(), 512);
        0
    }
}

pub fn read(lba: u64, count: u32, buf: &mut [u8]) -> u32 {
    for i in 0..count {
        let mut sector = [0u8; 512];
        if read_sector(lba + i as u64, &mut sector) != 0 {
            return 1;
        }
        let dst = (i as usize) * 512;
        buf[dst..dst + 512].copy_from_slice(&sector);
    }
    0
}

pub fn write_sector(lba: u64, buf: &[u8; 512]) -> u32 {
    unsafe {
        if ABAR == 0 {
            return 1;
        }
        core::ptr::copy_nonoverlapping(buf.as_ptr(), CMD_BUF.as_mut_ptr(), 512);
        let cl_base = (ABAR + PORT_BASE) as *const u32;
        let mut cl_phys = cl_base.read_volatile() as usize;
        if cl_phys & 0x1 != 0 {
            return 1;
        }
        cl_phys &= !0x3FF;
        let cmd = cl_phys as *mut u8;
        (cmd as *mut u32).write_volatile(0x10005);
        (cmd.add(4) as *mut u32).write_volatile(0);
        (cmd.add(8) as *mut u32).write_volatile((cl_phys + 0x80) as u32);
        (cmd.add(12) as *mut u32).write_volatile(0);
        let fis = cmd.add(0x80);
        fis.write_volatile(0x27);
        fis.add(1).write_volatile(0x80);
        fis.add(2).write_volatile(0x35);
        fis.add(3).write_volatile(0);
        fis.add(4).write_volatile(lba as u8);
        fis.add(5).write_volatile((lba >> 8) as u8);
        fis.add(6).write_volatile((lba >> 16) as u8);
        fis.add(7).write_volatile(0xE0);
        fis.add(8).write_volatile((lba >> 24) as u8);
        fis.add(9).write_volatile((lba >> 32) as u8);
        fis.add(10).write_volatile((lba >> 40) as u8);
        fis.add(11).write_volatile(0);
        fis.add(12).write_volatile(1);
        fis.add(13).write_volatile(0);
        fis.add(14).write_volatile(0);
        fis.add(15).write_volatile(0);
        fis.add(16).write_volatile(0);
        fis.add(17).write_volatile(0);
        fis.add(18).write_volatile(0);
        fis.add(19).write_volatile(0);
        let prdt = cmd.add(0x100);
        let data_phys = CMD_BUF.as_ptr() as usize - 0xFFFF800000000000;
        (prdt as *mut u64).write_volatile(data_phys as u64);
        prdt.add(8).write_volatile(0);
        (prdt.add(12) as *mut u32).write_volatile(0x1FF);
        let ci = (ABAR + PORT_BASE + 0x38) as *mut u32;
        ci.write_volatile(1);
        let mut t = 0;
        while ci.read_volatile() & 1 != 0 {
            t += 1;
            if t > 10000000 {
                crate::println!("ahci: write timeout lba={}", lba);
                return 1;
            }
            core::hint::spin_loop();
        }
        let is = (ABAR + PORT_BASE + 0x10) as *mut u32;
        is.write_volatile(is.read_volatile());
        0
    }
}

pub fn write(lba: u64, count: u32, buf: &[u8]) -> u32 {
    for i in 0..count {
        let src = (i as usize) * 512;
        let mut sector = [0u8; 512];
        sector.copy_from_slice(&buf[src..src + 512]);
        if write_sector(lba + i as u64, &sector) != 0 {
            return 1;
        }
    }
    0
}
