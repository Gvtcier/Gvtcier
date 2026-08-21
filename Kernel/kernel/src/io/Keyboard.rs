const RING_SIZE: usize = 128;

static mut RING: [u8; RING_SIZE] = [0; RING_SIZE];
static mut HEAD: usize = 0;
static mut TAIL: usize = 0;
static mut SHIFT: bool = false;
static mut CAPS: bool = false;

const NORMAL: [u8; 128] = [
    0, 0, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 0, 0,
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', 0, 0, b'a', b's',
    b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\', b'z', b'x', b'c', b'v',
    b'b', b'n', b'm', b',', b'.', b'/', 0, b'*', 0, b' ', 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const SHIFTED: [u8; 128] = [
    0, 0, b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')', b'_', b'+', 0, 0,
    b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P', b'{', b'}', 0, 0, b'A', b'S',
    b'D', b'F', b'G', b'H', b'J', b'K', b'L', b':', b'"', b'~', 0, b'|', b'Z', b'X', b'C', b'V',
    b'B', b'N', b'M', b'<', b'>', b'?', 0, b'*', 0, b' ', 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn push(sc: u8) {
    unsafe {
        if sc == 0x2A || sc == 0x36 {
            SHIFT = true;
            return;
        }
        if sc == 0xAA || sc == 0xB6 {
            SHIFT = false;
            return;
        }
        if sc == 0x3A {
            CAPS = !CAPS;
            return;
        }
        if sc & 0x80 != 0 {
            return;
        }
        if sc >= 128 {
            return;
        }
        let c = if sc == 0x1C {
            b'\r'
        } else if sc == 0x0E {
            8
        } else {
            let table = if SHIFT ^ CAPS { &SHIFTED } else { &NORMAL };
            table[sc as usize]
        };
        if c == 0 {
            return;
        }
        let next = (HEAD + 1) % RING_SIZE;
        if next == TAIL {
            return;
        }
        RING[HEAD] = crate::io::Gv2280::ascii_to_gv(c);
        HEAD = next;
    }
}

pub fn read_ready() -> bool {
    unsafe { HEAD != TAIL }
}

pub fn read_byte() -> u8 {
    unsafe {
        let b = RING[TAIL];
        TAIL = (TAIL + 1) % RING_SIZE;
        b
    }
}
