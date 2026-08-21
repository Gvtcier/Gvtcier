pub const CAP_FAT: u32 = 1;
pub const CAP_GPU: u32 = 2;
pub const CAP_AUDIO: u32 = 4;
const MAX_DRIVERS: usize = 16;

#[repr(C)]
pub struct Driver {
    pub caps: u32,
    pub ep: u32,
    pub task: u32,
}

const DRIVER_NONE: Driver = Driver {
    caps: 0,
    ep: 0,
    task: 0,
};

static mut DRIVERS: [Driver; MAX_DRIVERS] = [DRIVER_NONE; MAX_DRIVERS];

pub fn register(caps: u32, ep: u32, task: u32) -> u32 {
    unsafe {
        for i in 0..MAX_DRIVERS {
            if DRIVERS[i].caps != 0 && DRIVERS[i].caps == caps {
                DRIVERS[i] = Driver { caps, ep, task };
                return 0;
            }
        }
        for i in 0..MAX_DRIVERS {
            if DRIVERS[i].caps == 0 {
                DRIVERS[i] = Driver { caps, ep, task };
                return 0;
            }
        }
    }
    1
}

pub fn unregister(caps: u32) -> u32 {
    unsafe {
        for i in 0..MAX_DRIVERS {
            if DRIVERS[i].caps == caps {
                DRIVERS[i] = DRIVER_NONE;
                return 0;
            }
        }
    }
    1
}

pub fn lookup(caps: u32) -> i64 {
    unsafe {
        for i in 0..MAX_DRIVERS {
            if DRIVERS[i].caps != 0 && DRIVERS[i].caps & caps != 0 {
                return DRIVERS[i].ep as i64;
            }
        }
    }
    -1
}

pub fn count() -> u32 {
    unsafe {
        let mut n: u32 = 0;
        for i in 0..MAX_DRIVERS {
            if DRIVERS[i].caps != 0 {
                n += 1;
            }
        }
        n
    }
}
