#![no_std]
#![no_main]

mod Common;

use Common::*;

#[no_mangle]
static mut KBD_BUF: [u8; 32] = [0; 32];

#[no_mangle]
extern "C" fn user_main() -> ! {
    loop {
        let r = sys_recv(2, unsafe { &mut KBD_BUF }.as_ptr() as usize);
        if r > 0 {
            let b = unsafe { KBD_BUF[1] };
            print("kbd sc=");
            print_hex(b as u64);
            print(" r=");
            print_hex(r);
            print("\n");
        }
    }
}
