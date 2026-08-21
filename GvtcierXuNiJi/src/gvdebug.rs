pub const OP_READ: u8 = 1;
pub const OP_WRITE: u8 = 2;

pub struct Watchpoint {
    pub addr: u64,
    pub len: usize,
    pub kind: u8,
    pub name: &'static str,
    pub val: Option<u32>,
    pub hits: u32,
}

pub struct GvDebug {
    pub watchpoints: Vec<Watchpoint>,
    pub trace: Vec<String>,
    pub trace_max: usize,
    pub step: u32,
}

impl GvDebug {
    pub fn new() -> Self {
        GvDebug {
            watchpoints: Vec::new(),
            trace: Vec::new(),
            trace_max: 256,
            step: 0,
        }
    }

    pub fn add_watch(&mut self, addr: u64, len: usize, kind: u8, name: &'static str) {
        self.watchpoints.push(Watchpoint {
            addr,
            len,
            kind,
            name,
            val: None,
            hits: 0,
        });
    }

    pub fn add_watch_val(&mut self, addr: u64, len: usize, kind: u8, name: &'static str, val: u32) {
        self.watchpoints.push(Watchpoint {
            addr,
            len,
            kind,
            name,
            val: Some(val),
            hits: 0,
        });
    }

    pub fn check(&mut self, addr: u64, len: usize, op: u8, val: &[u8]) -> bool {
        let mut hit = false;
        for w in self.watchpoints.iter_mut() {
            if addr + len as u64 > w.addr && addr < w.addr + w.len as u64 && w.kind & op != 0 {
                if let Some(exp) = w.val {
                    if val.len() < 4 {
                        continue;
                    }
                    let actual = u32::from_le_bytes([val[0], val[1], val[2], val[3]]);
                    if actual != exp {
                        continue;
                    }
                }
                w.hits += 1;
                hit = true;
            }
        }
        hit
    }

    pub fn trace_add(&mut self, s: String) {
        self.step += 1;
        self.trace.push(format!("[{:04}] {}", self.step, s));
        if self.trace.len() > self.trace_max {
            self.trace.remove(0);
        }
    }

    pub fn dump(&self, mem: &[u8], addr: u64, len: usize) -> String {
        let mut out = String::new();
        let start = addr as usize;
        let end = core::cmp::min(start + len, mem.len());
        for off in (start..end).step_by(16) {
            let mut line = format!("{:08x}: ", off);
            for i in 0..16 {
                if off + i < end {
                    line.push_str(&format!("{:02x} ", mem[off + i]));
                } else {
                    line.push_str("   ");
                }
            }
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    pub fn report(&self) -> String {
        let mut out = String::new();
        out.push_str("== GvDebug 报告 ==\n");
        for w in &self.watchpoints {
            out.push_str(&format!("监控 {} 命中 {} 次\n", w.name, w.hits));
        }
        out.push_str(&format!("轨迹 {} 条\n", self.trace.len()));
        for t in &self.trace {
            out.push_str(&format!("  {}\n", t));
        }
        out
    }
}
