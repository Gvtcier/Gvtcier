pub mod Ahci;
pub mod Ata;
pub mod Audio;
pub mod Flac;
pub mod Ogg;
pub mod Mp3;
pub mod Fb;
pub mod File;
pub mod Gv2280;
pub mod Gvinter;
pub mod Keyboard;
pub mod Print;
pub mod Serial;
pub mod Shell;

pub fn outb(port: u16, v: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack));
    }
}
