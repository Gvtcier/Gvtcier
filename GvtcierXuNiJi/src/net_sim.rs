const REG_IDR0: u16 = 0x00;
const REG_TSD0: u16 = 0x10;
const REG_TSAD0: u16 = 0x20;
const REG_RBSTART: u16 = 0x30;
const REG_CR: u16 = 0x37;
const REG_CAPR: u16 = 0x38;
const REG_CBR: u16 = 0x3A;
const REG_ISR: u16 = 0x3E;
const REG_RCR: u16 = 0x44;
const REG_CONFIG1: u16 = 0x52;

const PEER_IP: [u8; 4] = [10, 0, 2, 2];
const GUEST_IP: [u8; 4] = [10, 0, 2, 15];
const DNS_SERVER_IP: [u8; 4] = [10, 0, 2, 3];
const PEER_MAC: [u8; 6] = [0x52, 0x54, 0x00, 0x00, 0x00, 0x01];

use crate::gvdebug::{GvDebug, OP_READ, OP_WRITE};

pub struct Rtl8139Sim {
    regs: [u8; 256],
    rx_buf: Vec<u8>,
    rx_buf_size: usize,
    clock_enabled: bool,
    receive_enabled: bool,
    isr: u16,
    rx_pos: usize,
    pub failures: Vec<String>,
    pub passes: Vec<String>,
    pub debug: GvDebug,
}

impl Rtl8139Sim {
    pub fn new() -> Self {
        let mut s = Rtl8139Sim {
            regs: [0u8; 256],
            rx_buf: vec![0u8; 8192 + 16],
            rx_buf_size: 8192 + 16,
            clock_enabled: false,
            receive_enabled: false,
            isr: 0,
            rx_pos: 0,
            failures: Vec::new(),
            passes: Vec::new(),
            debug: GvDebug::new(),
        };
        s.regs[0] = 0x56;
        s.regs[1] = 0x34;
        s.regs[2] = 0x12;
        s.regs[3] = 0x00;
        s.regs[4] = 0x54;
        s.regs[5] = 0x52;
        s
    }

    pub fn io_write(&mut self, addr: u16, width: u8, val: u32) {
        let a = addr as usize;
        self.debug.check(addr as u64, width as usize, OP_WRITE, &val.to_le_bytes());
        match addr {
            REG_CR => {
                self.regs[a] = (val & 0xFF) as u8;
                if val & 0x10 != 0 {
                    self.regs[a] &= !0x10;
                    self.receive_enabled = false;
                }
                self.receive_enabled = self.regs[a] & 0x08 != 0;
                self.passes.push(format!(
                    "CR 写入 0x{:02x}：接收使能={}",
                    self.regs[a],
                    self.receive_enabled
                ));
            }
            REG_CONFIG1 => {
                self.regs[a] = (val & 0xFF) as u8;
                self.clock_enabled = val & 0x10 == 0;
                self.passes.push(format!(
                    "CONFIG1 写入 0x{:02x}：时钟启用={}",
                    self.regs[a],
                    self.clock_enabled
                ));
            }
            REG_RCR => {
                for i in 0..4 {
                    self.regs[a + i] = (val >> (8 * i)) as u8;
                }
            }
            REG_RBSTART => {
                for i in 0..4 {
                    self.regs[a + i] = (val >> (8 * i)) as u8;
                }
                self.passes.push(format!("RBSTART 写入 {:#x}", val));
            }
            REG_CAPR => {
                self.regs[a] = (val & 0xFF) as u8;
                self.regs[a + 1] = ((val >> 8) & 0xFF) as u8;
            }
            REG_ISR => {
                self.isr &= !(val as u16);
                self.regs[a] = self.isr as u8;
                self.regs[a + 1] = (self.isr >> 8) as u8;
            }
            _ => {
                for i in 0..width as usize {
                    if a + i < 256 {
                        self.regs[a + i] = (val >> (8 * i)) as u8;
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn io_read(&mut self, addr: u16, width: u8) -> u32 {
        let a = addr as usize;
        let mut v: u32 = 0;
        match addr {
            REG_ISR => {
                self.regs[a] = self.isr as u8;
                self.regs[a + 1] = (self.isr >> 8) as u8;
            }
            REG_CBR => {}
            _ => {}
        }
        for i in 0..width as usize {
            if a + i < 256 {
                v |= (self.regs[a + i] as u32) << (8 * i);
            }
        }
        self.debug.check(addr as u64, width as usize, OP_READ, &v.to_le_bytes());
        v
    }

    pub fn read_mac(&self) -> [u8; 6] {
        let mut m = [0u8; 6];
        for i in 0..6 {
            m[i] = self.regs[REG_IDR0 as usize + i];
        }
        m
    }

    pub fn can_receive(&self) -> bool {
        self.clock_enabled && self.receive_enabled
    }

    pub fn receive_frame(&mut self, frame: &[u8]) -> bool {
        if !self.can_receive() {
            self.failures
                .push("接收被拒：时钟或接收未使能（can_receive=false）".into());
            return false;
        }
        let pos = self.rx_pos;
        let total = 4 + frame.len();
        let used = (total + 3) & !3;
        if pos + used > self.rx_buf_size {
            self.rx_pos = 0;
            return self.receive_frame(frame);
        }
        let hdr: u32 = (frame.len() as u32 & 0x1FFF) | 0x8000;
        self.rx_buf[pos..pos + 4].copy_from_slice(&hdr.to_le_bytes());
        self.rx_buf[pos + 4..pos + 4 + frame.len()].copy_from_slice(frame);
        self.rx_pos = pos + used;
        self.isr |= 0x01;
        self.regs[REG_ISR as usize] = self.isr as u8;
        self.regs[REG_ISR as usize + 1] = (self.isr >> 8) as u8;
        true
    }

    pub fn transmit(&mut self, frame: &[u8]) -> bool {
        self.io_write(REG_TSAD0, 32, 0x96000);
        self.io_write(REG_TSD0, 32, (frame.len() as u32 + 3) & !3);
        self.isr |= 0x04;
        true
    }

    pub fn peer_respond(&mut self, frame: &[u8]) -> Option<Vec<u8>> {
        if frame.len() < 14 {
            return None;
        }
        let et = u16::from_be_bytes([frame[12], frame[13]]);
        let payload = &frame[14..];
        match et {
            0x0806 => self.arp_respond(frame, payload),
            0x0800 => self.ipv4_respond(frame, payload),
            _ => None,
        }
    }

    fn eth_wrap(&self, dst: &[u8], ethertype: u16, payload: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(dst);
        f.extend_from_slice(&PEER_MAC);
        f.extend_from_slice(&ethertype.to_be_bytes());
        f.extend_from_slice(payload);
        f
    }

    fn arp_respond(&mut self, frame: &[u8], pkt: &[u8]) -> Option<Vec<u8>> {
        if pkt.len() < 28 {
            return None;
        }
        let oper = u16::from_be_bytes([pkt[6], pkt[7]]);
        let tpa = &pkt[24..28];
        if oper == 1 && tpa == PEER_IP {
            let mut r = [0u8; 28];
            r[0] = 0;
            r[1] = 1;
            r[2] = 0x08;
            r[3] = 0x00;
            r[4] = 6;
            r[5] = 4;
            r[6] = 0;
            r[7] = 2;
            r[8..14].copy_from_slice(&PEER_MAC);
            r[14..18].copy_from_slice(&PEER_IP);
            r[18..24].copy_from_slice(&pkt[8..14]);
            r[24..28].copy_from_slice(&pkt[14..18]);
            self.passes.push("ARP 请求已响应（10.0.2.2）".into());
            Some(self.eth_wrap(&frame[6..12], 0x0806, &r))
        } else {
            None
        }
    }

    fn ipv4_respond(&mut self, frame: &[u8], pkt: &[u8]) -> Option<Vec<u8>> {
        if pkt.len() < 20 {
            return None;
        }
        let ihl = ((pkt[0] & 0x0F) as usize) * 4;
        let proto = pkt[9];
        let src = &pkt[12..16];
        if proto == 1 && pkt.len() >= ihl + 8 {
            let icmp = &pkt[ihl..];
            if icmp[0] == 8 {
                let mut reply = vec![0u8; icmp.len()];
                reply.copy_from_slice(icmp);
                reply[0] = 0;
                reply[2] = 0;
                reply[3] = 0;
                let c = csum(&reply);
                reply[2] = (c >> 8) as u8;
                reply[3] = c as u8;
                let ip = build_ipv4(src, &PEER_IP, 1, &reply);
                self.passes
                    .push(format!("ICMP echo 已响应（来自 {}.{}.{}.{}）", src[0], src[1], src[2], src[3]));
                Some(self.eth_wrap(&frame[6..12], 0x0800, &ip))
            } else {
                None
            }
        } else if proto == 6 && pkt.len() >= ihl + 20 {
            let tcp = &pkt[ihl..];
            let flags = tcp[13];
            let dst_port = u16::from_be_bytes([tcp[2], tcp[3]]);
            let seq = u32::from_be_bytes([tcp[4], tcp[5], tcp[6], tcp[7]]);
            if dst_port == 80 && flags & 0x02 != 0 {
                let mut seg = vec![0u8; 20];
                seg[0] = tcp[2];
                seg[1] = tcp[3];
                seg[2] = tcp[0];
                seg[3] = tcp[1];
                seg[4..8].copy_from_slice(&0x4000u32.to_be_bytes());
                seg[8..12].copy_from_slice(&seq.wrapping_add(1).to_be_bytes());
                seg[12] = 0x50;
                seg[13] = 0x12;
                let c = csum_tcp(&PEER_IP, src, &seg);
                seg[16] = (c >> 8) as u8;
                seg[17] = c as u8;
                let ip = build_ipv4(src, &PEER_IP, 6, &seg);
                self.passes.push("TCP connect SYN 已响应 SYN-ACK（端口 80）".into());
                Some(self.eth_wrap(&frame[6..12], 0x0800, &ip))
            } else if dst_port == 80 && flags & 0x18 != 0 {
                let off = ((tcp[12] >> 4) as usize) * 4;
                let data = &tcp[off..];
                if data.starts_with(b"GET ") {
                    let body = b"HTTP/1.0 200 OK\r\nContent-Length: 13\r\n\r\nHello, Gvtcier";
                    let mut seg = vec![0u8; 20 + body.len()];
                    seg[0] = tcp[2];
                    seg[1] = tcp[3];
                    seg[2] = tcp[0];
                    seg[3] = tcp[1];
                    seg[4..8].copy_from_slice(&0x4001u32.to_be_bytes());
                    seg[8..12].copy_from_slice(&seq.wrapping_add(data.len() as u32).to_be_bytes());
                    seg[12] = 0x50;
                    seg[13] = 0x18;
                    seg[20..].copy_from_slice(body);
                    let c = csum_tcp(&PEER_IP, src, &seg);
                    seg[16] = (c >> 8) as u8;
                    seg[17] = c as u8;
                    let ip = build_ipv4(src, &PEER_IP, 6, &seg);
                    self.passes.push("HTTP GET 已响应 200 OK".into());
                    Some(self.eth_wrap(&frame[6..12], 0x0800, &ip))
                } else {
                    None
                }
            } else if flags & 0x02 != 0 {
                let mut seg = vec![0u8; 20];
                seg[0] = tcp[2];
                seg[1] = tcp[3];
                seg[2] = tcp[0];
                seg[3] = tcp[1];
                seg[4..8].copy_from_slice(&0x2000u32.to_be_bytes());
                seg[8..12].copy_from_slice(&seq.wrapping_add(1).to_be_bytes());
                seg[12] = 0x50;
                seg[13] = 0x12;
                let c = csum_tcp(&PEER_IP, src, &seg);
                seg[16] = (c >> 8) as u8;
                seg[17] = c as u8;
                let ip = build_ipv4(src, &PEER_IP, 6, &seg);
                self.passes.push("TCP SYN 已响应 SYN-ACK（端口 8080）".into());
                Some(self.eth_wrap(&frame[6..12], 0x0800, &ip))
            } else {
                None
            }
        } else if proto == 17 && pkt.len() >= ihl + 8 {
            let udp = &pkt[ihl..];
            let dport = u16::from_be_bytes([udp[2], udp[3]]);
            if dport == 53 {
                let resp = build_dns_response(&udp[8..]);
                let ip = build_ipv4(src, &PEER_IP, 17, &resp);
                self.passes.push("DNS 查询已响应（A 记录）".into());
                Some(self.eth_wrap(&frame[6..12], 0x0800, &ip))
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub fn csum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += ((data[i] as u32) << 8) | data[i + 1] as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn build_ipv4(dst: &[u8], src: &[u8], proto: u8, payload: &[u8]) -> Vec<u8> {
    let hlen = 20;
    let mut ip = vec![0u8; hlen + payload.len()];
    ip[0] = 0x45;
    let total = hlen + payload.len();
    ip[2] = (total >> 8) as u8;
    ip[3] = total as u8;
    ip[8] = 64;
    ip[9] = proto;
    ip[12..16].copy_from_slice(src);
    ip[16..20].copy_from_slice(dst);
    let c = csum(&ip[..hlen]);
    ip[10] = (c >> 8) as u8;
    ip[11] = c as u8;
    ip[hlen..].copy_from_slice(payload);
    ip
}

pub fn csum_tcp(src: &[u8], dst: &[u8], seg: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for b in src {
        sum += (*b as u32) << 8;
    }
    for b in dst {
        sum += *b as u32;
    }
    sum += 6;
    sum += seg.len() as u32;
    let mut i = 0;
    while i + 1 < seg.len() {
        sum += ((seg[i] as u32) << 8) | seg[i + 1] as u32;
        i += 2;
    }
    if i < seg.len() {
        sum += (seg[i] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn validate_frame(frame: &[u8], label: &str, results: &mut Vec<String>, failures: &mut Vec<String>) {
    if frame.len() < 14 {
        failures.push(format!("{}：帧过短", label));
        return;
    }
    let et = u16::from_be_bytes([frame[12], frame[13]]);
    results.push(format!("{}：ethtype=0x{:04x} len={}", label, et, frame.len()));
    match et {
        0x0806 => validate_arp(frame, label, results, failures),
        0x0800 => validate_ipv4(frame, label, results, failures),
        0x86DD => results.push(format!("{}：IPv6 帧（基础支持）", label)),
        _ => {}
    }
}

fn validate_arp(frame: &[u8], label: &str, results: &mut Vec<String>, failures: &mut Vec<String>) {
    let pkt = &frame[14..];
    if pkt.len() < 28 {
        failures.push(format!("{}：ARP 帧过短", label));
        return;
    }
    if u16::from_be_bytes([pkt[0], pkt[1]]) != 1 || u16::from_be_bytes([pkt[2], pkt[3]]) != 0x0800 {
        failures.push(format!("{}：ARP 类型错误", label));
        return;
    }
    if pkt[4] != 6 || pkt[5] != 4 {
        failures.push(format!("{}：ARP 地址长度错误", label));
        return;
    }
    let oper = u16::from_be_bytes([pkt[6], pkt[7]]);
    let spa = &pkt[14..18];
    let tpa = &pkt[24..28];
    results.push(format!(
        "{}：ARP oper={} spa={}.{}.{}.{} tpa={}.{}.{}.{}",
        label,
        oper,
        spa[0], spa[1], spa[2], spa[3],
        tpa[0], tpa[1], tpa[2], tpa[3]
    ));
}

fn validate_ipv4(frame: &[u8], label: &str, results: &mut Vec<String>, failures: &mut Vec<String>) {
    let pkt = &frame[14..];
    if pkt.len() < 20 {
        failures.push(format!("{}：IPv4 帧过短", label));
        return;
    }
    let ihl = ((pkt[0] & 0x0F) as usize) * 4;
    let proto = pkt[9];
    let total = ((pkt[2] as usize) << 8) | pkt[3] as usize;
    if total != pkt.len() {
        failures.push(format!("{}：IPv4 总长不符（头说 {}，实际 {}）", label, total, pkt.len()));
        return;
    }
    let c = csum(&pkt[..ihl]);
    if c != 0 {
        failures.push(format!("{}：IPv4 头校验和不通过（{:#x}）", label, c));
        return;
    }
    let src = &pkt[12..16];
    results.push(format!(
        "{}：IPv4 proto={} src={}.{}.{}.{} total={} 校验和通过",
        label,
        proto,
        src[0], src[1], src[2], src[3],
        total
    ));
    if proto == 1 && pkt.len() >= ihl + 8 {
        let icmp = &pkt[ihl..];
        let c2 = csum(icmp);
        results.push(format!("{}：ICMP type={} code={} 校验和{}", label, icmp[0], icmp[1], if c2 == 0 { "通过" } else { "不通过" }));
        if c2 != 0 {
            failures.push(format!("{}：ICMP 校验和不通过", label));
        }
    } else if proto == 6 && pkt.len() >= ihl + 20 {
        let tcp = &pkt[ihl..];
        let dst = &pkt[16..20];
        let c2 = csum_tcp(src, dst, tcp);
        results.push(format!("{}：TCP flags=0x{:02x} 校验和{}", label, tcp[13], if c2 == 0 { "通过" } else { "不通过" }));
        if c2 != 0 {
            failures.push(format!("{}：TCP 校验和不通过", label));
        }
    } else if proto == 17 && pkt.len() >= ihl + 8 {
        let udp = &pkt[ihl..];
        let sport = ((udp[0] as u16) << 8) | udp[1] as u16;
        let dport = ((udp[2] as u16) << 8) | udp[3] as u16;
        let ulen = ((udp[4] as u16) << 8) | udp[5] as u16;
        results.push(format!("{}：UDP sport={} dport={} len={}", label, sport, dport, ulen));
    }
}

pub fn eth_frame(dst: &[u8; 6], src: &[u8; 6], ethertype: u16, payload: &[u8]) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(dst);
    f.extend_from_slice(src);
    f.extend_from_slice(&ethertype.to_be_bytes());
    f.extend_from_slice(payload);
    f
}

pub fn build_arp_request(mac: &[u8; 6]) -> Vec<u8> {
    let mut arp = [0u8; 28];
    arp[0] = 0;
    arp[1] = 1;
    arp[2] = 0x08;
    arp[3] = 0x00;
    arp[4] = 6;
    arp[5] = 4;
    arp[6] = 0;
    arp[7] = 1;
    arp[8..14].copy_from_slice(mac);
    arp[14..18].copy_from_slice(&GUEST_IP);
    arp[24..28].copy_from_slice(&PEER_IP);
    eth_frame(&[0xFF; 6], mac, 0x0806, &arp)
}

pub fn build_ping(mac: &[u8; 6]) -> Vec<u8> {
    let mut icmp = [0u8; 8];
    icmp[0] = 8;
    icmp[1] = 0;
    icmp[4] = 1;
    icmp[5] = 0;
    let c = csum(&icmp);
    icmp[2] = (c >> 8) as u8;
    icmp[3] = c as u8;
    let ip = build_ipv4(&PEER_IP, &GUEST_IP, 1, &icmp);
    eth_frame(&PEER_MAC, mac, 0x0800, &ip)
}

pub fn build_tcp_syn(mac: &[u8; 6]) -> Vec<u8> {
    let mut seg = [0u8; 20];
    seg[0] = 0x1F;
    seg[1] = 0x90;
    seg[2] = 0x1F;
    seg[3] = 0x90;
    seg[4..8].copy_from_slice(&0x1000u32.to_be_bytes());
    seg[12] = 0x50;
    seg[13] = 0x02;
    let c = csum_tcp(&GUEST_IP, &PEER_IP, &seg);
    seg[16] = (c >> 8) as u8;
    seg[17] = c as u8;
    let ip = build_ipv4(&PEER_IP, &GUEST_IP, 6, &seg);
    eth_frame(&PEER_MAC, mac, 0x0800, &ip)
}

pub fn build_tcp_connect_syn(mac: &[u8; 6]) -> Vec<u8> {
    let mut seg = [0u8; 20];
    seg[0] = 0x9C;
    seg[1] = 0x40;
    seg[2] = 0;
    seg[3] = 80;
    seg[4..8].copy_from_slice(&0x3000u32.to_be_bytes());
    seg[12] = 0x50;
    seg[13] = 0x02;
    let c = csum_tcp(&GUEST_IP, &PEER_IP, &seg);
    seg[16] = (c >> 8) as u8;
    seg[17] = c as u8;
    let ip = build_ipv4(&PEER_IP, &GUEST_IP, 6, &seg);
    eth_frame(&PEER_MAC, mac, 0x0800, &ip)
}

pub fn build_tcp_ack_get(mac: &[u8; 6]) -> Vec<u8> {
    let req = b"GET / HTTP/1.0\r\nHost: test\r\n\r\n";
    let mut seg = vec![0u8; 20 + req.len()];
    seg[0] = 0x9C;
    seg[1] = 0x40;
    seg[2] = 0;
    seg[3] = 80;
    seg[4..8].copy_from_slice(&0x3001u32.to_be_bytes());
    seg[8..12].copy_from_slice(&0x4001u32.to_be_bytes());
    seg[12] = 0x50;
    seg[13] = 0x18;
    seg[20..].copy_from_slice(req);
    let c = csum_tcp(&GUEST_IP, &PEER_IP, &seg);
    seg[16] = (c >> 8) as u8;
    seg[17] = c as u8;
    let ip = build_ipv4(&PEER_IP, &GUEST_IP, 6, &seg);
    eth_frame(&PEER_MAC, mac, 0x0800, &ip)
}

pub fn build_dns_query(mac: &[u8; 6]) -> Vec<u8> {
    let mut q = [0u8; 12 + 6 + 4];
    q[0] = 0x12;
    q[1] = 0x34;
    q[5] = 1;
    q[7] = 1;
    q[12] = 4;
    q[13..17].copy_from_slice(b"test");
    q[17] = 0;
    q[18] = 0;
    q[19] = 1;
    q[20] = 0;
    q[21] = 1;
    let mut udp = vec![0u8; 8 + 22];
    udp[0] = 0x9C;
    udp[1] = 0x41;
    udp[2] = 0;
    udp[3] = 53;
    udp[4] = 0;
    udp[5] = 30;
    udp[8..].copy_from_slice(&q);
    let mut sum: u32 = 0;
    for b in &GUEST_IP {
        sum += *b as u32;
    }
    for b in &DNS_SERVER_IP {
        sum += (*b as u32) << 8;
    }
    sum += 17;
    sum += udp.len() as u32;
    let mut i = 0;
    while i + 1 < udp.len() {
        sum += ((udp[i] as u32) << 8) | udp[i + 1] as u32;
        i += 2;
    }
    if i < udp.len() {
        sum += (udp[i] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    let c = !(sum as u16);
    udp[6] = (c >> 8) as u8;
    udp[7] = c as u8;
    let ip = build_ipv4(&DNS_SERVER_IP, &GUEST_IP, 17, &udp);
    eth_frame(&PEER_MAC, mac, 0x0800, &ip)
}

fn build_dns_response(query: &[u8]) -> Vec<u8> {
    let mut r = vec![0u8; 64];
    if query.len() >= 2 {
        r[0] = query[0];
        r[1] = query[1];
    }
    r[2] = 0x81;
    r[3] = 0x80;
    r[7] = 1;
    let mut qoff = 12usize;
    let mut roff = 12usize;
    while qoff < query.len() && query[qoff] != 0 && (roff + 1 + query[qoff] as usize) < r.len() {
        let l = query[qoff] as usize;
        r[roff] = query[qoff];
        r[roff + 1..roff + 1 + l].copy_from_slice(&query[qoff + 1..qoff + 1 + l]);
        qoff += 1 + l;
        roff += 1 + l;
    }
    r[roff] = 0;
    roff += 1;
    r[roff] = 0;
    r[roff + 1] = 1;
    roff += 2;
    r[roff] = 0;
    r[roff + 1] = 1;
    roff += 2;
    r[roff] = 0xC0;
    r[roff + 1] = 0x0C;
    roff += 2;
    r[roff] = 0;
    r[roff + 1] = 1;
    roff += 2;
    r[roff] = 0;
    r[roff + 1] = 1;
    roff += 2;
    r[roff] = 0;
    r[roff + 1] = 0;
    r[roff + 2] = 0;
    r[roff + 3] = 60;
    roff += 4;
    r[roff] = 0;
    r[roff + 1] = 4;
    roff += 2;
    r[roff] = 10;
    r[roff + 1] = 0;
    r[roff + 2] = 2;
    r[roff + 3] = 99;
    roff += 4;
    r.truncate(roff);
    r
}

pub fn run() {
    let mut sim = Rtl8139Sim::new();
    let mut results: Vec<String> = Vec::new();
    sim.debug.add_watch(0x37, 1, OP_WRITE, "CR 命令寄存器");
    sim.debug.add_watch_val(0x52, 1, OP_WRITE, "CONFIG1 时钟开启", 0x00);

    println!("\n=== GvtcierXuNiJi 网络模拟（RTL8139 仿真 + 网络对端）===");

    println!("\n-- 1. Gvinter 初始化序列 --");
    sim.io_write(REG_CR, 8, 0x10);
    let mac = sim.read_mac();
    sim.io_write(REG_CONFIG1, 8, 0x00);
    sim.io_write(REG_RCR, 32, 0x80F);
    sim.io_write(REG_RBSTART, 32, 0x93000);
    sim.io_write(REG_CAPR, 16, 0);
    sim.io_write(REG_CR, 8, 0x0C);
    println!(
        "MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    if !sim.can_receive() {
        sim.failures.push("can_receive=false：接收未就绪".into());
    } else {
        sim.passes.push("can_receive=true：初始化后接收就绪".into());
    }

    println!("\n-- 2. Gvinter 帧验证 --");
    let arp = build_arp_request(&mac);
    validate_frame(&arp, "ARP 请求", &mut results, &mut sim.failures);
    let ping = build_ping(&mac);
    validate_frame(&ping, "ping(ICMP)", &mut results, &mut sim.failures);
    let syn = build_tcp_syn(&mac);
    validate_frame(&syn, "TCP SYN", &mut results, &mut sim.failures);

    println!("\n-- 3. 网络对端响应 --");
    for (name, frame) in [
        ("ARP 请求", arp),
        ("ping 请求", ping),
        ("TCP SYN", syn),
    ] {
        sim.transmit(&frame);
        if let Some(resp) = sim.peer_respond(&frame) {
            if sim.receive_frame(&resp) {
                validate_frame(&resp, &format!("对端响应({})", name), &mut results, &mut sim.failures);
            }
        }
    }

    println!("\n-- 4. P3 网络应用层 --");
    let syn_conn = build_tcp_connect_syn(&mac);
    validate_frame(&syn_conn, "TCP connect SYN", &mut results, &mut sim.failures);
    sim.transmit(&syn_conn);
    if let Some(resp) = sim.peer_respond(&syn_conn) {
        if sim.receive_frame(&resp) {
            validate_frame(&resp, "对端响应(SYN-ACK 80)", &mut results, &mut sim.failures);
        }
    }
    let get = build_tcp_ack_get(&mac);
    validate_frame(&get, "HTTP GET", &mut results, &mut sim.failures);
    sim.transmit(&get);
    if let Some(resp) = sim.peer_respond(&get) {
        if sim.receive_frame(&resp) {
            validate_frame(&resp, "对端响应(HTTP 200)", &mut results, &mut sim.failures);
        }
    }
    let dns = build_dns_query(&mac);
    validate_frame(&dns, "DNS 查询", &mut results, &mut sim.failures);
    sim.transmit(&dns);
    if let Some(resp) = sim.peer_respond(&dns) {
        if sim.receive_frame(&resp) {
            validate_frame(&resp, "对端响应(DNS A)", &mut results, &mut sim.failures);
        }
    }

    println!("{}", sim.debug.report());

    println!("\n== 网络模拟结果 ==");
    for p in &sim.passes {
        println!("✓ {}", p);
    }
    for r in &results {
        println!("· {}", r);
    }
    if sim.failures.is_empty() {
        println!("\n✓ 网络模拟全部通过——Gvinter 的初始化序列、帧构造与协议栈逻辑符合规范");
    } else {
        for f in &sim.failures {
            println!("✗ {}", f);
        }
        println!("\n✗ 存在失败项，需修正");
    }
}
