use crate::io::Serial;

const CMD_MAX: usize = 64;
const ARG_MAX: usize = 4;

pub struct Command {
    pub name: &'static str,
    pub desc: &'static str,
    pub usage: &'static str,
    pub handler: fn(args: &[&str]) -> u32,
}

pub static COMMANDS: &[Command] = &[
    Command {
        name: "help",
        desc: "show command list or detail",
        usage: "help [cmd]",
        handler: cmd_help,
    },
    Command {
        name: "info",
        desc: "system information",
        usage: "info",
        handler: cmd_info,
    },
    Command {
        name: "tasks",
        desc: "list tasks",
        usage: "tasks",
        handler: cmd_tasks,
    },
    Command {
        name: "kill",
        desc: "terminate a task",
        usage: "kill <tid>",
        handler: cmd_kill,
    },
    Command {
        name: "meminfo",
        desc: "memory statistics",
        usage: "meminfo",
        handler: cmd_meminfo,
    },
    Command {
        name: "dump",
        desc: "hexdump memory",
        usage: "dump <addr> <n>",
        handler: cmd_dump,
    },
    Command {
        name: "reboot",
        desc: "soft reset",
        usage: "reboot",
        handler: cmd_reboot,
    },
    Command {
        name: "halt",
        desc: "halt cpu",
        usage: "halt",
        handler: cmd_halt,
    },
    Command {
        name: "backtrace",
        desc: "stack backtrace",
        usage: "backtrace",
        handler: cmd_backtrace,
    },
    Command {
        name: "log",
        desc: "dump kernel log ring",
        usage: "log",
        handler: cmd_log,
    },
    Command {
        name: "write",
        desc: "file write test (TEST.TXT)",
        usage: "write",
        handler: cmd_write,
    },
    Command {
        name: "ls",
        desc: "list files",
        usage: "ls",
        handler: cmd_ls,
    },
    Command {
        name: "cat",
        desc: "print file content",
        usage: "cat <file>",
        handler: cmd_cat,
    },
    Command {
        name: "mkdir",
        desc: "create directory",
        usage: "mkdir <dir>",
        handler: cmd_mkdir,
    },
    Command {
        name: "rm",
        desc: "remove file",
        usage: "rm <file>",
        handler: cmd_rm,
    },
    Command {
        name: "cd",
        desc: "change directory",
        usage: "cd <dir>",
        handler: cmd_cd,
    },
    Command {
        name: "ping",
        desc: "icmp echo to ip",
        usage: "ping <ip>",
        handler: cmd_ping,
    },
    Command {
        name: "dns",
        desc: "resolve host name",
        usage: "dns <name>",
        handler: cmd_dns,
    },
    Command {
        name: "http",
        desc: "http get request",
        usage: "http <host> <path>",
        handler: cmd_http,
    },
    Command {
        name: "netstat",
        desc: "show tcp state",
        usage: "netstat",
        handler: cmd_netstat,
    },
    Command {
        name: "time",
        desc: "show rtc time and uptime",
        usage: "time",
        handler: cmd_time,
    },
    Command {
        name: "echo",
        desc: "print arguments",
        usage: "echo <text>",
        handler: cmd_echo,
    },
    Command {
        name: "pwd",
        desc: "show current directory",
        usage: "pwd",
        handler: cmd_pwd,
    },
    Command {
        name: "ver",
        desc: "show kernel version",
        usage: "ver",
        handler: cmd_ver,
    },
];

fn cmd_help(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("GvShell:\r\n");
            for c in COMMANDS {
                Serial::print_str(c.name);
                Serial::print_str("   ");
                Serial::print_str(c.desc);
                Serial::print_str("\r\n");
            }
        } else {
            for c in COMMANDS {
                if c.name == args[0] {
                    Serial::print_str(c.usage);
                    Serial::print_str("\r\n");
                    return 0;
                }
            }
            Serial::print_str("unknown command\r\n");
        }
    }
    0
}

fn cmd_info(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("Gvtcier Kernel v0.1\r\n");
        Serial::print_str("uptime: ");
        Serial::print_hex(crate::intr::Apic::tick());
        Serial::print_str(" ticks\r\n");
    }
    0
}

fn cmd_tasks(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("tid pid state prio\r\n");
        crate::Task::shell_tasks();
    }
    0
}

fn cmd_kill(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("usage: kill <tid>\r\n");
            return 1;
        }
        let tid = parse_hex(args[0]);
        crate::Task::shell_kill(tid as u32);
    }
    0
}

fn cmd_meminfo(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("memory: see meminfo (buddy stats)\r\n");
    }
    0
}

fn cmd_dump(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("usage: dump <addr> <n>\r\n");
            return 1;
        }
        let addr = parse_hex(args[0]) as usize;
        let mut n = 16usize;
        if args.len() > 1 {
            n = parse_hex(args[1]) as usize;
        }
        if n > 256 {
            n = 256;
        }
        if addr < 0xFFFF800000000000 || addr.saturating_add(n) > 0xFFFF800100000000 {
            Serial::print_str("dump: addr out of kernel range\r\n");
            return 1;
        }
        for i in 0..n {
            if i % 16 == 0 {
                Serial::print_hex((addr + i) as u64);
                Serial::print_str(": ");
            }
            let b = *((addr + i) as *const u8);
            Serial::print_hex2(b);
            Serial::print_str(" ");
            if i % 16 == 15 {
                Serial::print_str("\r\n");
            }
        }
        Serial::print_str("\r\n");
    }
    0
}

fn cmd_reboot(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("rebooting...\r\n");
        crate::io::outb(0x64, 0xFE);
        loop {
            core::hint::spin_loop();
        }
    }
}

fn cmd_halt(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("system halted\r\n");
        loop {
            core::arch::asm!("cli");
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

fn cmd_backtrace(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("backtrace:\r\n");
        let mut rbp: usize;
        core::arch::asm!("mov {}, rbp", out(reg) rbp);
        for i in 0..32 {
            if rbp < 0xFFFF800000000000 || rbp > 0xFFFF800100000000 {
                break;
            }
            let ret = *(rbp as *const usize).add(1);
            Serial::print_hex(ret as u64);
            Serial::print_str("\r\n");
            rbp = *(rbp as *const usize);
        }
    }
    0
}

fn cmd_log(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("kernel log:\r\n");
        crate::io::Print::log_dump();
    }
    0
}

fn cmd_write(_args: &[&str]) -> u32 {
    unsafe {
        let name = b"TEST    TXT";
        let h = crate::io::File::open(name);
        if h == 0xFFFFFFFF {
            Serial::print_str("write: open fail\r\n");
            return 1;
        }
        let data = b"Gvtcier write test v0.2";
        let mut buf = [0u8; 64];
        let n = crate::io::File::write(h, data.as_ptr(), data.len());
        Serial::print_str("write: wrote ");
        Serial::print_hex(n as u64);
        Serial::print_str(" bytes\r\n");
        crate::io::File::close(h);
        let h2 = crate::io::File::open(name);
        if h2 != 0xFFFFFFFF {
            let r = crate::io::File::read(h2, buf.as_mut_ptr(), 64);
            Serial::print_str("write: read back ");
            Serial::print_hex(r as u64);
            Serial::print_str(": ");
            for i in 0..r {
                Serial::write_byte(buf[i as usize]);
            }
            Serial::print_str("\r\n");
            crate::io::File::close(h2);
        }
    }
    0
}

fn cmd_ls(_args: &[&str]) -> u32 {
    unsafe {
        let mut buf = [0u8; 512];
        let n = crate::io::File::list(buf.as_mut_ptr(), buf.len());
        Serial::write_bytes(&buf[..n as usize]);
        Serial::print_str("\r\n");
    }
    0
}

fn cmd_cat(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("usage: cat <file>\r\n");
            return 1;
        }
        let h = crate::io::File::open(args[0].as_bytes());
        if h == 0xFFFFFFFF {
            Serial::print_str("cat: not found\r\n");
            return 1;
        }
        let mut buf = [0u8; 512];
        let r = crate::io::File::read(h, buf.as_mut_ptr(), buf.len());
        Serial::write_bytes(&buf[..r as usize]);
        Serial::print_str("\r\n");
        crate::io::File::close(h);
    }
    0
}

fn cmd_mkdir(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("usage: mkdir <dir>\r\n");
            return 1;
        }
        let r = crate::io::File::mkdir(args[0].as_bytes());
        if r != 0 {
            Serial::print_str("mkdir: fail\r\n");
        } else {
            Serial::print_str("mkdir: ok\r\n");
        }
    }
    0
}

fn cmd_rm(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("usage: rm <file>\r\n");
            return 1;
        }
        let r = crate::io::File::remove(args[0].as_bytes());
        if r != 0 {
            Serial::print_str("rm: fail\r\n");
        } else {
            Serial::print_str("rm: ok\r\n");
        }
    }
    0
}

fn cmd_cd(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("usage: cd <dir>\r\n");
            return 1;
        }
        let r = crate::io::File::cd(args[0].as_bytes());
        if r != 0 {
            Serial::print_str("cd: fail\r\n");
        } else {
            Serial::print_str("cd: ok\r\n");
        }
    }
    0
}

fn parse_ip(s: &str) -> Option<[u8; 4]> {
    let mut parts = s.split('.');
    let mut ip = [0u8; 4];
    for i in 0..4 {
        let p = parts.next()?;
        let v: u32 = p.parse().ok()?;
        if v > 255 {
            return None;
        }
        ip[i] = v as u8;
    }
    Some(ip)
}

fn cmd_ping(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("usage: ping <ip>\r\n");
            return 1;
        }
        let ip = match parse_ip(args[0]) {
            Some(ip) => ip,
            None => {
                Serial::print_str("ping: bad ip\r\n");
                return 1;
            }
        };
        let r = crate::io::Gvinter::ping(&ip);
        if r == 0 {
            Serial::print_str("ping: reply ok\r\n");
        } else {
            Serial::print_str("ping: no reply\r\n");
        }
    }
    0
}

fn cmd_dns(args: &[&str]) -> u32 {
    unsafe {
        if args.is_empty() {
            Serial::print_str("usage: dns <name>\r\n");
            return 1;
        }
        let r = crate::io::Gvinter::dns_query(args[0].as_bytes());
        if r != 0 {
            Serial::print_str("dns: send fail\r\n");
            return 1;
        }
        let mut t = 0;
        while !crate::io::Gvinter::dns_done() && t < 1000000 {
            crate::io::Gvinter::poll();
            t += 1;
            core::hint::spin_loop();
        }
        if crate::io::Gvinter::dns_done() {
            let ip = crate::io::Gvinter::dns_result();
            Serial::print_str("dns: ");
            Serial::print_hex(ip[0] as u64);
            Serial::print_str(".");
            Serial::print_hex(ip[1] as u64);
            Serial::print_str(".");
            Serial::print_hex(ip[2] as u64);
            Serial::print_str(".");
            Serial::print_hex(ip[3] as u64);
            Serial::print_str("\r\n");
        } else {
            Serial::print_str("dns: no answer\r\n");
        }
    }
    0
}

fn cmd_http(args: &[&str]) -> u32 {
    unsafe {
        if args.len() < 2 {
            Serial::print_str("usage: http <host> <path>\r\n");
            return 1;
        }
        let r = crate::io::Gvinter::http_get(args[0].as_bytes(), args[1].as_bytes());
        if r != 0 {
            Serial::print_str("http: fail\r\n");
            return 1;
        }
        let len = crate::io::Gvinter::http_data_len();
        Serial::print_str("http: ");
        Serial::print_hex(len as u64);
        Serial::print_str(" bytes\r\n");
        let ptr = crate::io::Gvinter::http_data_ptr();
        for i in 0..len {
            Serial::write_byte(*ptr.add(i));
        }
        Serial::print_str("\r\n");
    }
    0
}

fn cmd_netstat(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("tcp state: ");
        Serial::print_hex(crate::io::Gvinter::net_state() as u64);
        Serial::print_str("\r\n");
    }
    0
}

fn cmd_time(_args: &[&str]) -> u32 {
    unsafe {
        let (h, m, s) = crate::Time::now();
        Serial::print_str("time: ");
        Serial::print_hex(h as u64);
        Serial::print_str(":");
        Serial::print_hex(m as u64);
        Serial::print_str(":");
        Serial::print_hex(s as u64);
        Serial::print_str("  uptime: ");
        Serial::print_hex(crate::intr::Apic::tick());
        Serial::print_str(" ticks\r\n");
    }
    0
}

fn cmd_echo(args: &[&str]) -> u32 {
    unsafe {
        for a in args {
            Serial::print_str(a);
            Serial::print_str(" ");
        }
        Serial::print_str("\r\n");
    }
    0
}

fn cmd_pwd(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("cwd cluster: ");
        Serial::print_hex(crate::io::File::cwd() as u64);
        Serial::print_str("\r\n");
    }
    0
}

fn cmd_ver(_args: &[&str]) -> u32 {
    unsafe {
        Serial::print_str("Gvtcier Kernel v0.7\r\n");
    }
    0
}

pub fn parse_hex(s: &str) -> u64 {
    let mut v: u64 = 0;
    for c in s.bytes() {
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => break,
        };
        v = v * 16 + d as u64;
    }
    v
}

pub fn selftest() {
    unsafe {
        Serial::print_str("=== GvShell selftest ===\r\n");
        cmd_ls(&[]);
        cmd_cat(&["A.TXT"]);
        cmd_mkdir(&["DOCS"]);
        cmd_cd(&["DOCS"]);
        cmd_ls(&[]);
        cmd_cd(&[".."]);
        cmd_rm(&["C.TXT"]);
        cmd_ls(&[]);
        Serial::print_str("=== GvShell selftest done ===\r\n");
    }
}

fn handle_input(buf: &mut [u8], len: &mut usize, b: u8) {
    unsafe {
        let c = crate::io::Gv2280::gv_to_ascii(b);
        if c == b'\r' || c == b'\n' {
            Serial::print_str("\r\n");
            if *len > 0 {
                buf[*len] = 0;
                let line = core::str::from_utf8_unchecked(&buf[..*len]);
                let (args, n) = split_args(line);
                let mut done = false;
                if n > 0 {
                    for c in COMMANDS {
                        if c.name == args[0] {
                            (c.handler)(&args[1..n]);
                            done = true;
                            break;
                        }
                    }
                    if !done {
                        Serial::print_str("unknown: ");
                        Serial::print_str(args[0]);
                        Serial::print_str("\r\n");
                    }
                }
                *len = 0;
            }
        } else if c == 8 || c == 0x7F {
            if *len > 0 {
                *len -= 1;
                Serial::print_str("\x08 \x08");
            }
        } else if *len < CMD_MAX - 1 {
            buf[*len] = c;
            *len += 1;
            Serial::write_byte(b);
        }
    }
}

pub fn run() -> ! {
    unsafe {
        Serial::print_str("GvShell. type help\r\n");
        let mut buf = [0u8; CMD_MAX];
        let mut len = 0usize;
        loop {
            if Serial::read_ready() {
                let b = Serial::read_byte();
                handle_input(&mut buf, &mut len, crate::io::Gv2280::ascii_to_gv(b));
            }
            if crate::io::Keyboard::read_ready() {
                let b = crate::io::Keyboard::read_byte();
                handle_input(&mut buf, &mut len, b);
            }
        }
    }
}

fn split_args(line: &str) -> ([&str; ARG_MAX], usize) {
    let mut args = [""; ARG_MAX];
    let mut n = 0usize;
    for part in line.split_whitespace() {
        if n >= ARG_MAX {
            break;
        }
        args[n] = part;
        n += 1;
    }
    (args, n)
}
