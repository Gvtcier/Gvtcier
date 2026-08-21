use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub struct Spinlock {
    locked: AtomicBool,
    owner: AtomicU32,
}

impl Spinlock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            owner: AtomicU32::new(0),
        }
    }

    pub fn lock(&self) {
        loop {
            if !self.locked.swap(true, Ordering::Acquire) {
                self.owner.store(crate::Task::current_id(), Ordering::Relaxed);
                return;
            }
            while self.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
    }

    pub fn try_lock(&self) -> bool {
        if !self.locked.swap(true, Ordering::Acquire) {
            self.owner.store(crate::Task::current_id(), Ordering::Relaxed);
            return true;
        }
        false
    }

    pub fn unlock(&self) {
        self.owner.store(0, Ordering::Relaxed);
        self.locked.store(false, Ordering::Release);
    }

    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    pub fn owner(&self) -> u32 {
        self.owner.load(Ordering::Relaxed)
    }
}
