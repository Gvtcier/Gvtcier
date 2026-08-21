pub const MAX_DEVICES: usize = 64;

#[repr(C)]
pub struct Device {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor: u16,
    pub device_id: u16,
    pub class: u16,
    pub irq: u8,
    pub present: bool,
}

const DEVICE_NONE: Device = Device {
    bus: 0,
    dev: 0,
    func: 0,
    vendor: 0,
    device_id: 0,
    class: 0,
    irq: 0,
    present: false,
};

static mut DEVICES: [Device; MAX_DEVICES] = [DEVICE_NONE; MAX_DEVICES];

unsafe fn inl(port: u16) -> u32 {
    let v: u32;
    core::arch::asm!("in eax, dx", out("eax") v, in("dx") port, options(nomem, nostack));
    v
}

unsafe fn outl(port: u16, v: u32) {
    core::arch::asm!("out dx, eax", in("dx") port, in("eax") v, options(nomem, nostack));
}

const PCI_ADDR: u16 = 0xCF8;
const PCI_DATA: u16 = 0xCFC;

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

fn is_multi(dev: u8) -> bool {
    pci_read(0, dev, 0, 0x0C) & 0x00800000 != 0
}

pub fn scan() -> u32 {
    let mut n: u32 = 0;
    unsafe {
        for dev in 0..32 {
            let id = pci_read(0, dev, 0, 0);
            if id == 0xFFFFFFFF {
                continue;
            }
            let vendor = (id & 0xFFFF) as u16;
            let device_id = ((id >> 16) & 0xFFFF) as u16;
            let class = (pci_read(0, dev, 0, 0x08) >> 16) as u16;
            let irq = (pci_read(0, dev, 0, 0x3C) & 0xFF) as u8;
            let max_func = if is_multi(dev) { 8 } else { 1 };
            for func in 0..max_func {
                let fid = pci_read(0, dev, func as u8, 0);
                if fid == 0xFFFFFFFF {
                    continue;
                }
                if n < MAX_DEVICES as u32 {
                    DEVICES[n as usize] = Device {
                        bus: 0,
                        dev: dev as u8,
                        func: func as u8,
                        vendor,
                        device_id: ((fid >> 16) & 0xFFFF) as u16,
                        class: (pci_read(0, dev, func as u8, 0x08) >> 16) as u16,
                        irq: (pci_read(0, dev, func as u8, 0x3C) & 0xFF) as u8,
                        present: true,
                    };
                    n += 1;
                }
            }
        }
    }
    n
}

pub fn count() -> u32 {
    unsafe {
        let mut n: u32 = 0;
        for i in 0..MAX_DEVICES {
            if DEVICES[i].present {
                n += 1;
            }
        }
        n
    }
}

pub fn get(i: usize) -> Option<&'static Device> {
    unsafe {
        if i < MAX_DEVICES && DEVICES[i].present {
            Some(&DEVICES[i])
        } else {
            None
        }
    }
}

pub fn bar(bus: u8, dev: u8, func: u8, index: u8) -> u32 {
    if index > 5 {
        return 0;
    }
    pci_read(bus, dev, func, 0x10 + index * 4)
}

pub fn poll() -> u32 {
    unsafe {
        let mut changes: u32 = 0;
        let mut seen = [false; MAX_DEVICES];
        let mut idx = 0usize;
        for dev in 0..32u8 {
            let id = pci_read(0, dev, 0, 0);
            if id == 0xFFFFFFFF {
                continue;
            }
            let max_func = if is_multi(dev) { 8 } else { 1 };
            for func in 0..max_func {
                let fid = pci_read(0, dev, func as u8, 0);
                if fid == 0xFFFFFFFF {
                    continue;
                }
                if idx < MAX_DEVICES {
                    seen[idx] = true;
                    if !DEVICES[idx].present {
                        DEVICES[idx] = Device {
                            bus: 0,
                            dev: dev,
                            func: func as u8,
                            vendor: (id & 0xFFFF) as u16,
                            device_id: ((fid >> 16) & 0xFFFF) as u16,
                            class: (pci_read(0, dev, func as u8, 0x08) >> 16) as u16,
                            irq: (pci_read(0, dev, func as u8, 0x3C) & 0xFF) as u8,
                            present: true,
                        };
                        crate::println!(
                            "pci: added bus=0 dev={} func={} ven={:#x} did={:#x}",
                            dev,
                            func,
                            (id & 0xFFFF) as u16,
                            ((fid >> 16) & 0xFFFF) as u16
                        );
                        changes += 1;
                    }
                    idx += 1;
                }
            }
        }
        for i in 0..MAX_DEVICES {
            if DEVICES[i].present && !seen[i] {
                crate::println!(
                    "pci: removed bus=0 dev={} func={}",
                    DEVICES[i].dev,
                    DEVICES[i].func
                );
                DEVICES[i] = DEVICE_NONE;
                changes += 1;
            }
        }
        changes
    }
}
