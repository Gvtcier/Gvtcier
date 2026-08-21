pub const MSG_SIZE: usize = 32;
pub const QUEUE_CAP: usize = 64;
pub const MAX_ENDPOINTS: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Message {
    pub len: u32,
    pub data: [u8; MSG_SIZE],
}

#[repr(C)]
pub struct Endpoint {
    pub owner: u32,
    pub head: usize,
    pub tail: usize,
    pub queue: [Message; QUEUE_CAP],
}

const ENDPOINT_NONE: Endpoint = Endpoint {
    owner: 0,
    head: 0,
    tail: 0,
    queue: [Message {
        len: 0,
        data: [0; MSG_SIZE],
    }; QUEUE_CAP],
};

static mut ENDPOINTS: [Endpoint; MAX_ENDPOINTS] = [ENDPOINT_NONE; MAX_ENDPOINTS];

pub fn create(owner: u32) -> u32 {
    unsafe {
        for i in 0..MAX_ENDPOINTS {
            if ENDPOINTS[i].owner == 0 {
                ENDPOINTS[i].owner = owner;
                ENDPOINTS[i].head = 0;
                ENDPOINTS[i].tail = 0;
                return i as u32;
            }
        }
    }
    0xFFFFFFFF
}

pub fn destroy(ep: u32) -> u32 {
    unsafe {
        if (ep as usize) >= MAX_ENDPOINTS {
            return 1;
        }
        let _ifs = irq_save();
        ENDPOINTS[ep as usize] = ENDPOINT_NONE;
        irq_restore(_ifs);
    }
    0
}

pub fn send(ep: u32, msg: &Message) -> u32 {
    unsafe {
        if (ep as usize) >= MAX_ENDPOINTS {
            return 1;
        }
        let _ifs = irq_save();
        let e = &mut ENDPOINTS[ep as usize];
        let next = (e.tail + 1) % QUEUE_CAP;
        if next == e.head {
            irq_restore(_ifs);
            return 1;
        }
        e.queue[e.tail] = *msg;
        e.tail = next;
        irq_restore(_ifs);
        crate::Task::wake_on(ep);
    }
    0
}

pub fn recv(ep: u32, out: &mut Message) -> u32 {
    unsafe {
        if (ep as usize) >= MAX_ENDPOINTS {
            return 1;
        }
        let _ifs = irq_save();
        let e = &mut ENDPOINTS[ep as usize];
        if e.head == e.tail {
            irq_restore(_ifs);
            return 1;
        }
        *out = e.queue[e.head];
        e.head = (e.head + 1) % QUEUE_CAP;
        irq_restore(_ifs);
    }
    0
}

pub fn used(ep: u32) -> u32 {
    unsafe {
        if (ep as usize) >= MAX_ENDPOINTS {
            return 0;
        }
        let e = &ENDPOINTS[ep as usize];
        if e.tail >= e.head {
            (e.tail - e.head) as u32
        } else {
            (QUEUE_CAP - e.head + e.tail) as u32
        }
    }
}

pub fn free(ep: u32) -> u32 {
    unsafe {
        if (ep as usize) >= MAX_ENDPOINTS {
            return 0;
        }
        let e = &ENDPOINTS[ep as usize];
        if e.tail >= e.head {
            (QUEUE_CAP - e.tail + e.head) as u32
        } else {
            (e.head - e.tail) as u32
        }
    }
}

pub fn endpoint_count() -> u32 {
    unsafe {
        let mut n: u32 = 0;
        for i in 0..MAX_ENDPOINTS {
            if ENDPOINTS[i].owner != 0 {
                n += 1;
            }
        }
        n
    }
}

unsafe fn irq_save() -> u64 {
    let r: u64;
    core::arch::asm!("pushfq", "pop {0}", out(reg) r, options(nomem, nostack));
    core::arch::asm!("cli", options(nomem, nostack));
    r
}

unsafe fn irq_restore(r: u64) {
    if r & 0x200 != 0 {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}
