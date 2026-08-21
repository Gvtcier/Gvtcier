const MAX_SEMS: usize = 8;
const MAX_CVS: usize = 8;

static mut SEMS: [(i32, u32, bool); MAX_SEMS] = [(0, 0, false); MAX_SEMS];
static mut CVS: [(u32, bool); MAX_CVS] = [(0, false); MAX_CVS];

pub fn sem_create(count: i32) -> u32 {
    unsafe {
        for i in 0..MAX_SEMS {
            if !SEMS[i].2 {
                let ep = crate::Ipc::create(0);
                SEMS[i] = (count, ep, true);
                return i as u32;
            }
        }
    }
    0xFFFFFFFF
}

pub fn sem_wait(h: u32) {
    unsafe {
        if (h as usize) >= MAX_SEMS || !SEMS[h as usize].2 {
            return;
        }
        SEMS[h as usize].0 -= 1;
        if SEMS[h as usize].0 < 0 {
            crate::Task::block_on(SEMS[h as usize].1);
        }
    }
}

pub fn sem_post(h: u32) {
    unsafe {
        if (h as usize) >= MAX_SEMS || !SEMS[h as usize].2 {
            return;
        }
        SEMS[h as usize].0 += 1;
        if SEMS[h as usize].0 <= 0 {
            crate::Task::wake_on(SEMS[h as usize].1);
        }
    }
}

pub fn sem_destroy(h: u32) {
    unsafe {
        if (h as usize) < MAX_SEMS {
            SEMS[h as usize].2 = false;
        }
    }
}

pub fn cv_create() -> u32 {
    unsafe {
        for i in 0..MAX_CVS {
            if !CVS[i].1 {
                let ep = crate::Ipc::create(0);
                CVS[i] = (ep, true);
                return i as u32;
            }
        }
    }
    0xFFFFFFFF
}

pub fn cv_wait(h: u32) {
    unsafe {
        if (h as usize) >= MAX_CVS || !CVS[h as usize].1 {
            return;
        }
        crate::Task::block_on(CVS[h as usize].0);
    }
}

pub fn cv_notify(h: u32) {
    unsafe {
        if (h as usize) >= MAX_CVS || !CVS[h as usize].1 {
            return;
        }
        crate::Task::wake_on(CVS[h as usize].0);
    }
}

pub fn cv_destroy(h: u32) {
    unsafe {
        if (h as usize) < MAX_CVS {
            CVS[h as usize].1 = false;
        }
    }
}
