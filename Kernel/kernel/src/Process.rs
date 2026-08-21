use crate::Cap::{Cap, CAP_NONE, MAX_CAPS};

pub const MAX_PROCESSES: usize = 64;

#[derive(Clone, Copy, PartialEq)]
pub enum ProcessState {
    Dead,
    Alive,
}

#[repr(C)]
pub struct Process {
    pub id: u32,
    pub parent: u32,
    pub state: ProcessState,
    pub run_ticks: u64,
    pub caps: [Cap; MAX_CAPS],
}

const PROCESS_NONE: Process = Process {
    id: 0,
    parent: 0,
    state: ProcessState::Dead,
    run_ticks: 0,
    caps: [CAP_NONE; MAX_CAPS],
};

static mut PROCESSES: [Process; MAX_PROCESSES] = [PROCESS_NONE; MAX_PROCESSES];
static mut NEXT_PID: u32 = 1;

pub fn fission(caps: [Cap; MAX_CAPS]) -> u32 {
    unsafe {
        for i in 0..MAX_PROCESSES {
            if PROCESSES[i].state == ProcessState::Dead {
                PROCESSES[i] = Process {
                    id: NEXT_PID,
                    parent: crate::Task::current_id(),
                    state: ProcessState::Alive,
                    run_ticks: 0,
                    caps,
                };
                NEXT_PID += 1;
                return PROCESSES[i].id;
            }
        }
    }
    0
}

pub fn terminate(pid: u32) {
    unsafe {
        for i in 0..MAX_PROCESSES {
            if PROCESSES[i].id == pid && PROCESSES[i].state == ProcessState::Alive {
                PROCESSES[i].state = ProcessState::Dead;
            }
        }
    }
}

pub fn count() -> u32 {
    unsafe {
        let mut n: u32 = 0;
        for i in 0..MAX_PROCESSES {
            if PROCESSES[i].state == ProcessState::Alive {
                n += 1;
            }
        }
        n
    }
}

pub fn get(pid: u32) -> Option<&'static Process> {
    unsafe {
        for i in 0..MAX_PROCESSES {
            if PROCESSES[i].id == pid && PROCESSES[i].state == ProcessState::Alive {
                return Some(&PROCESSES[i]);
            }
        }
    }
    None
}

pub fn tick(pid: u32) {
    unsafe {
        for i in 0..MAX_PROCESSES {
            if PROCESSES[i].id == pid && PROCESSES[i].state == ProcessState::Alive {
                PROCESSES[i].run_ticks += 1;
                return;
            }
        }
    }
}

pub fn total_ticks() -> u64 {
    unsafe {
        let mut t: u64 = 0;
        for i in 0..MAX_PROCESSES {
            if PROCESSES[i].state == ProcessState::Alive {
                t += PROCESSES[i].run_ticks;
            }
        }
        t
    }
}
