#![no_std]
#![no_main]

mod Common;

use Common::*;

#[no_mangle]
static mut MSG_BUF: [u8; 32] = [0; 32];

#[no_mangle]
extern "C" fn user_main() -> ! {
    unsafe {
        loop {
            let r = sys_recv(4, MSG_BUF.as_ptr() as usize);
            if r >= 4 {
                let kind = MSG_BUF[0];
                let b = MSG_BUF[1];
                let dx = MSG_BUF[2] as i8;
                let dy = MSG_BUF[3] as i8;
                print("mouse: kind=");
                print_hex(kind as u64);
                print(" dx=");
                print_hex(dx as u64);
                print(" dy=");
                print_hex(dy as u64);
                print(" b=");
                print_hex(b as u64);
                print("\n");
            }
        }
    }
}
