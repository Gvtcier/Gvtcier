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

static mut NABM: u16 = 0;

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
    for dev in 0..32 {
        let id = pci_read(0, dev, 5, 0);
        let vid = id & 0xFFFF;
        let did = (id >> 16) & 0xFFFF;
        if vid == 0x8086 && (did == 0x2415 || did == 0x2425 || did == 0x2445 || did == 0x2668) {
            let bar = pci_read(0, dev, 5, 0x10);
            unsafe {
                NABM = (bar & 0xFFFFFFF0) as u16;
            }
            ac97_reset();
            ac97_set_rate(48000);
            ac97_set_master(0);
            return 0;
        }
    }
    1
}

fn ac97_write(reg: u8, v: u16) {
    unsafe {
        let n = NABM as usize;
        if NABM == 0 {
            return;
        }
        let addr = n + 0x80;
        *(addr as *mut u16).add((reg / 2) as usize) = v;
    }
}

fn ac97_reset() {
    ac97_write(0x00, 0x0000);
}

fn ac97_set_rate(rate: u32) {
    ac97_write(0x2C, rate as u16);
}

fn ac97_set_master(vol: u8) {
    let v = (((vol & 0x1F) as u16) << 8) | (vol & 0x1F) as u16;
    ac97_write(0x02, v);
}

pub fn nabm() -> u16 {
    unsafe { NABM }
}

pub struct HypInfo {
    pub sample_rate: u32,
    pub bits: u32,
    pub channels: u32,
    pub data_len: u32,
    pub data_off: u32,
}

pub fn hyp_parse(data: &[u8]) -> Option<HypInfo> {
    if data.len() < 24 || &data[0..3] != b"HYP" {
        return None;
    }
    let sample_rate = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let bits = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    let channels = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    let data_len = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
    if 24 + data_len as usize > data.len() {
        return None;
    }
    Some(HypInfo {
        sample_rate,
        bits,
        channels,
        data_len,
        data_off: 24,
    })
}

use super::Flac;

pub fn play_hyp(data: &[u8]) -> u32 {
    match hyp_parse(data) {
        Some(info) => {
            let pcm = &data[info.data_off as usize..(info.data_off + info.data_len) as usize];
            play_pcm(pcm.as_ptr(), pcm.len() as u32, info.sample_rate, info.bits, info.channels)
        }
        None => 1,
    }
}

pub fn play_audio(data: &[u8]) -> u32 {
    if data.len() >= 3 && &data[0..3] == b"HYP" {
        return play_hyp(data);
    }
    if data.len() >= 4 && &data[0..4] == b"fLaC" {
        let mut pcm: alloc::vec::Vec<i16> = alloc::vec::Vec::new();
        if let Some(info) = Flac::decode(data, &mut pcm) {
            return play_pcm(
                pcm.as_ptr() as *const u8,
                (pcm.len() * 2) as u32,
                info.sample_rate,
                info.bits as u32,
                info.channels as u32,
            );
        }
        return 1;
    }
    if data.len() >= 4 && &data[0..4] == b"OggS" {
        return 3;
    }
    if data.len() >= 2 && data[0] == 0xFF && data[1] & 0xE0 == 0xE0 {
        return 3;
    }
    1
}

pub fn play_pcm(ptr: *const u8, len: u32, sample_rate: u32, _bits: u32, _channels: u32) -> u32 {
    unsafe {
        let n = NABM as usize;
        if NABM == 0 || len == 0 {
            return 2;
        }
        if sample_rate > 0 {
            ac97_set_rate(sample_rate);
        }
        static mut BD: [u32; 4] = [0; 4];
        BD[0] = (ptr as u64 - 0xFFFF800000000000) as u32;
        BD[1] = 0x80000000 | (len & 0xFFFF);
        let bd_phys = (BD.as_ptr() as u64 - 0xFFFF800000000000) as u32;
        *(n as *mut u32).add(0x20 / 4) = bd_phys;
        *(n as *mut u16).add(0x1A / 2) = 0x1;
    }
    0
}

pub fn play(ptr: *const u8, len: u32) -> u32 {
    if ptr.is_null() || len == 0 {
        return 2;
    }
    let data = unsafe { core::slice::from_raw_parts(ptr, len as usize) };
    play_audio(data)
}
