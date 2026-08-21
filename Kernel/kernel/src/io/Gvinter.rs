use core::arch::asm;

const PCI_ADDR: u16 = 0xCF8;
const PCI_DATA: u16 = 0xCFC;

const REG_IDR0: u16 = 0x00;
const REG_TSD0: u16 = 0x10;
const REG_TSAD0: u16 = 0x20;
const REG_RBSTART: u16 = 0x30;
const REG_CR: u16 = 0x37;
const REG_CAPR: u16 = 0x38;
const REG_CBR: u16 = 0x3A;
const REG_IMR: u16 = 0x3C;
const REG_ISR: u16 = 0x3E;
const REG_RCR: u16 = 0x44;
const REG_CONFIG1: u16 = 0x52;

const RX_BUF_SIZE: usize = 8192 + 16;
const RX_BUF_PHYS: usize = 0x93000;
const TX_BUF_PHYS: usize = 0x96000;

const GATEWAY_IP: [u8; 4] = [10, 0, 2, 2];
const DNS_SERVER: [u8; 4] = [10, 0, 2, 3];
const DNS_PORT: u16 = 53;
const TCP_PORT: u16 = 8080;
const HTTP_SERVER_PORT: u16 = 80;
const DHCP_CLIENT_PORT: u16 = 68;
const DHCP_SERVER_PORT: u16 = 67;

static mut IO_BASE: u16 = 0;
static mut MAC: [u8; 6] = [0; 6];
static mut LINK_UP: bool = false;
static mut GW_MAC: [u8; 6] = [0xFF; 6];
static mut MY_IP: [u8; 4] = [10, 0, 2, 15];
static mut DHCP_STATE: u8 = 0;
static mut DHCP_XID: u32 = 0x2A11;
static mut DHCP_OFFER_IP: [u8; 4] = [0; 4];
static mut DHCP_SERVER: [u8; 4] = [0; 4];
static mut DHCP_DONE: bool = false;

const ARP_CACHE_MAX: usize = 8;
static mut ARP_CACHE_IP: [[u8; 4]; ARP_CACHE_MAX] = [[0; 4]; ARP_CACHE_MAX];
static mut ARP_CACHE_MAC: [[u8; 6]; ARP_CACHE_MAX] = [[0; 6]; ARP_CACHE_MAX];
static mut ARP_CACHE_VALID: [bool; ARP_CACHE_MAX] = [false; ARP_CACHE_MAX];
static mut ARP_CACHE_NEXT: usize = 0;
static mut ARP_PENDING_IP: [u8; 4] = [0; 4];
static mut ARP_PENDING_TICK: u64 = 0;
static mut ARP_PENDING_ACTIVE: bool = false;

fn arp_lookup(ip: &[u8]) -> Option<[u8; 6]> {
    unsafe {
        for i in 0..ARP_CACHE_MAX {
            if ARP_CACHE_VALID[i] && ARP_CACHE_IP[i] == ip {
                return Some(ARP_CACHE_MAC[i]);
            }
        }
    }
    None
}

fn arp_update(ip: &[u8], mac: &[u8]) {
    unsafe {
        for i in 0..ARP_CACHE_MAX {
            if ARP_CACHE_VALID[i] && ARP_CACHE_IP[i] == ip {
                ARP_CACHE_MAC[i].copy_from_slice(mac);
                return;
            }
        }
        let i = ARP_CACHE_NEXT % ARP_CACHE_MAX;
        ARP_CACHE_IP[i].copy_from_slice(ip);
        ARP_CACHE_MAC[i].copy_from_slice(mac);
        ARP_CACHE_VALID[i] = true;
        ARP_CACHE_NEXT = i + 1;
    }
}

static mut TCP_ESTABLISHED: bool = false;
static mut TCP_ACK: u32 = 0;
static mut TCP_REMOTE: [u8; 4] = [0; 4];
static mut TCP_REMOTE_PORT: u16 = 0;
static mut TCP_LOCAL_SEQ: u32 = 0x2000;

const MAX_CONNS: usize = 8;
const CONN_FREE: u8 = 0;
const CONN_SYN_SENT: u8 = 1;
const CONN_ESTABLISHED: u8 = 2;
const CONN_CLOSE_WAIT: u8 = 3;
const CONN_LISTEN: u8 = 4;

#[repr(C)]
pub struct Conn {
    pub state: u8,
    pub dir: u8,
    pub local_port: u16,
    pub remote: [u8; 4],
    pub remote_port: u16,
    pub send_seq: u32,
    pub recv_ack: u32,
    pub recv_buf: [u8; 2048],
    pub recv_len: usize,
    pub syn_tick: u64,
    pub retry: u8,
    pub cwnd: u32,
    pub ssthresh: u32,
}

const CONN_NONE: Conn = Conn {
    state: CONN_FREE,
    dir: 0,
    local_port: 0,
    remote: [0; 4],
    remote_port: 0,
    send_seq: 0,
    recv_ack: 0,
    recv_buf: [0; 2048],
    recv_len: 0,
    syn_tick: 0,
    retry: 0,
    cwnd: 1,
    ssthresh: 64,
};

static mut CONNS: [Conn; MAX_CONNS] = [CONN_NONE; MAX_CONNS];

fn conn_find(remote: &[u8; 4], remote_port: u16, local_port: u16) -> Option<usize> {
    unsafe {
        for i in 0..MAX_CONNS {
            let c = &CONNS[i];
            if c.state != CONN_FREE
                && c.remote == *remote
                && c.remote_port == remote_port
                && c.local_port == local_port
            {
                return Some(i);
            }
        }
    }
    None
}

fn conn_alloc() -> Option<usize> {
    unsafe {
        for i in 0..MAX_CONNS {
            if CONNS[i].state == CONN_FREE {
                CONNS[i] = CONN_NONE;
                return Some(i);
            }
        }
    }
    None
}

fn conn_free(i: usize) {
    unsafe {
        CONNS[i] = CONN_NONE;
    }
}

static mut TCP_STATE: u8 = 0;
static mut TCP_CONNECT_IP: [u8; 4] = [0; 4];
static mut TCP_CONNECT_PORT: u16 = 0;
static mut TCP_LOCAL_PORT: u16 = 40000;
static mut TCP_SEND_SEQ: u32 = 0;
static mut TCP_RECV_ACK: u32 = 0;
static mut TCP_RECV_BUF: [u8; 2048] = [0; 2048];
static mut TCP_RECV_LEN: usize = 0;
static mut TCP_SYN_TICK: u64 = 0;
static mut TCP_RETRY: u8 = 0;
static mut UDP_BOUND_PORT: u16 = 0;
static mut UDP_RECV_BUF: [u8; 2048] = [0; 2048];
static mut UDP_RECV_LEN: usize = 0;
static mut UDP_RECV_PORT: u16 = 0;
static mut DNS_PENDING: bool = false;
static mut DNS_ID: u16 = 0x1234;
static mut DNS_RESULT: [u8; 4] = [0; 4];
static mut DNS_DONE: bool = false;
static mut HTTP_BUF: [u8; 2048] = [0; 2048];
static mut HTTP_LEN: usize = 0;
static mut PING_REPLY: bool = false;

unsafe fn outb(port: u16, v: u8) {
    asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack));
}
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack));
    v
}
unsafe fn outw(port: u16, v: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") v, options(nomem, nostack));
}
unsafe fn inw(port: u16) -> u16 {
    let v: u16;
    asm!("in ax, dx", out("ax") v, in("dx") port, options(nomem, nostack));
    v
}
unsafe fn outl(port: u16, v: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") v, options(nomem, nostack));
}
unsafe fn inl(port: u16) -> u32 {
    let v: u32;
    asm!("in eax, dx", out("eax") v, in("dx") port, options(nomem, nostack));
    v
}

fn pci_read(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    unsafe {
        outl(
            PCI_ADDR,
            0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((off as u32) & 0xFC),
        );
        inl(PCI_DATA)
    }
}

fn pci_write(bus: u8, dev: u8, func: u8, off: u8, v: u32) {
    unsafe {
        outl(
            PCI_ADDR,
            0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((off as u32) & 0xFC),
        );
        outl(PCI_DATA, v);
    }
}

const RX_BUF_MEM: *mut u8 = RX_BUF_PHYS as *mut u8;
const TX_BUF_MEM: *mut u8 = TX_BUF_PHYS as *mut u8;

pub fn init() -> u32 {
    unsafe {
        for dev in 0..32 {
            for func in 0..8 {
                let vd = pci_read(0, dev, func, 0);
                let vendor = vd & 0xFFFF;
                let device = (vd >> 16) & 0xFFFF;
                let class = (pci_read(0, dev, func, 0x08) >> 16) & 0xFFFF;
                if class == 0x0200 && vendor == 0x10EC && device == 0x8139 {
                    let bar0 = pci_read(0, dev, func, 0x10) & !0x3;
                    IO_BASE = bar0 as u16;
                    pci_write(0, dev, func, 0x04, pci_read(0, dev, func, 0x04) | 0x5);
                    outb(IO_BASE + REG_CR, 0x10);
                    for _ in 0..1000 {
                        if inb(IO_BASE + REG_CR) & 0x10 == 0 {
                            break;
                        }
                    }
                    for i in 0..6 {
                        MAC[i] = inb(IO_BASE + REG_IDR0 + i as u16);
                    }
                    outb(IO_BASE + REG_CONFIG1, 0x00);
                    outl(IO_BASE + REG_RCR, 0x80F);
                    outl(IO_BASE + REG_RBSTART, RX_BUF_PHYS as u32);
                    outw(IO_BASE + REG_CAPR, 0);
                    outw(IO_BASE + REG_IMR, 0);
                    outw(IO_BASE + REG_ISR, 0xFFFF);
                    outb(IO_BASE + REG_CR, 0x0C);
                    LINK_UP = true;
                    crate::println!(
                        "gvinter: rtl8139 ready mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} ip={}.{}.{}.{} link=1",
                        MAC[0], MAC[1], MAC[2], MAC[3], MAC[4], MAC[5],
                        MY_IP[0], MY_IP[1], MY_IP[2], MY_IP[3]
                    );
                    return 0;
                }
            }
        }
    }
    1
}

unsafe fn send_frame(data: &[u8]) {
    if !LINK_UP || data.len() > 1500 {
        return;
    }
    core::ptr::copy_nonoverlapping(data.as_ptr(), TX_BUF_MEM, data.len());
    let len = (data.len() + 3) & !3;
    for i in data.len()..len {
        TX_BUF_MEM.add(i).write_volatile(0);
    }
    outl(IO_BASE + REG_TSAD0, TX_BUF_PHYS as u32);
    outl(IO_BASE + REG_TSD0, len as u32);
}

unsafe fn eth_send(dst: &[u8; 6], ethertype: u16, payload: &[u8]) {
    let mut frame = [0u8; 1518];
    let n = 14 + payload.len();
    frame[0..6].copy_from_slice(dst);
    frame[6..12].copy_from_slice(&MAC);
    frame[12] = (ethertype >> 8) as u8;
    frame[13] = ethertype as u8;
    frame[14..n].copy_from_slice(payload);
    send_frame(&frame[..n]);
}

fn csum(data: &[u8]) -> u16 {
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

const BCAST: [u8; 6] = [0xFF; 6];

unsafe fn arp_handle(pkt: &[u8]) {
    if pkt.len() < 28 {
        return;
    }
    if u16::from_be_bytes([pkt[0], pkt[1]]) != 1 || u16::from_be_bytes([pkt[2], pkt[3]]) != 0x0800 {
        return;
    }
    let oper = u16::from_be_bytes([pkt[6], pkt[7]]);
    let tpa = &pkt[24..28];
    if oper == 1 && tpa == MY_IP {
        arp_update(&pkt[14..18], &pkt[8..14]);
        let mut r = [0u8; 28];
        r[0] = 0;
        r[1] = 1;
        r[2] = 0x08;
        r[3] = 0x00;
        r[4] = 6;
        r[5] = 4;
        r[6] = 0;
        r[7] = 2;
        r[8..14].copy_from_slice(&MAC);
        r[14..18].copy_from_slice(&MY_IP);
        r[18..24].copy_from_slice(&pkt[8..14]);
        r[24..28].copy_from_slice(&pkt[14..18]);
        eth_send(&pkt[8..14].try_into().unwrap(), 0x0806, &r);
    } else if oper == 2 && &pkt[14..18] == GATEWAY_IP {
        GW_MAC.copy_from_slice(&pkt[8..14]);
        arp_update(&pkt[14..18], &pkt[8..14]);
        if ARP_PENDING_ACTIVE && ARP_PENDING_IP == pkt[14..18] {
            ARP_PENDING_ACTIVE = false;
        }
        crate::println!(
            "gvinter: gw mac {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            GW_MAC[0], GW_MAC[1], GW_MAC[2], GW_MAC[3], GW_MAC[4], GW_MAC[5]
        );
    } else if oper == 2 {
        arp_update(&pkt[14..18], &pkt[8..14]);
        if ARP_PENDING_ACTIVE && ARP_PENDING_IP == pkt[14..18] {
            ARP_PENDING_ACTIVE = false;
        }
    }
}

unsafe fn ipv4_handle(pkt: &[u8]) {
    if pkt.len() < 20 {
        return;
    }
    let hc = csum(&pkt[..20]);
    if hc != 0 {
        crate::println!("gvinter: bad ip csum={:#06x}", hc);
        return;
    }
    let ihl = ((pkt[0] & 0x0F) as usize) * 4;
    let proto = pkt[9];
    let src = &pkt[12..16];
    let dst = &pkt[16..20];
    if dst != MY_IP {
        return;
    }
    if proto == 1 {
        let icmp = &pkt[ihl..];
        if icmp.len() >= 8 && icmp[0] == 8 {
            let mut reply = [0u8; 64];
            let n = core::cmp::min(icmp.len(), 64);
            reply[..n].copy_from_slice(&icmp[..n]);
            reply[0] = 0;
            reply[2] = 0;
            reply[3] = 0;
            let c = csum(&reply[..n]);
            reply[2] = (c >> 8) as u8;
            reply[3] = c as u8;
            ipv4_send(src, 1, &reply[..n]);
        } else if icmp.len() >= 8 && icmp[0] == 0 {
            PING_REPLY = true;
            crate::println!(
                "gvinter: ping reply from {}.{}.{}.{}",
                src[0], src[1], src[2], src[3]
            );
        }
    } else if proto == 6 {
        tcp_handle(src, &pkt[ihl..]);
    } else if proto == 17 {
        udp_handle(src, &pkt[ihl..]);
    }
}

unsafe fn ipv4_send(dst: &[u8], proto: u8, payload: &[u8]) {
    let mut ip = [0u8; 60];
    let hlen = 20;
    ip[0] = 0x45;
    let total = hlen + payload.len();
    ip[2] = (total >> 8) as u8;
    ip[3] = total as u8;
    ip[8] = 64;
    ip[9] = proto;
    ip[12..16].copy_from_slice(&MY_IP);
    ip[16..20].copy_from_slice(dst);
    let c = csum(&ip[..hlen]);
    ip[10] = (c >> 8) as u8;
    ip[11] = c as u8;
    let mut pkt = [0u8; 1500];
    pkt[..hlen].copy_from_slice(&ip[..hlen]);
    pkt[hlen..total].copy_from_slice(payload);
    let mut dm = [0xFFu8; 6];
    unsafe {
        if let Some(mac) = arp_lookup(dst) {
            dm.copy_from_slice(&mac);
        } else if GW_MAC[0] != 0xFF {
            dm.copy_from_slice(&GW_MAC);
        }
    }
    eth_send(&dm, 0x0800, &pkt[..total]);
}

unsafe fn tcp_send_conn_synack(ci: usize, ack: u32) {
    unsafe {
        let c = &CONNS[ci];
        let mut seg = [0u8; 20];
        seg[0] = (c.local_port >> 8) as u8;
        seg[1] = c.local_port as u8;
        seg[2] = (c.remote_port >> 8) as u8;
        seg[3] = c.remote_port as u8;
        seg[4..8].copy_from_slice(&c.send_seq.to_be_bytes());
        seg[8..12].copy_from_slice(&ack.to_be_bytes());
        seg[12] = 0x50;
        seg[13] = 0x12;
        let chk = csum_tcp_to(&c.remote, &seg);
        seg[16] = (chk >> 8) as u8;
        seg[17] = chk as u8;
        ipv4_send(&c.remote, 6, &seg);
    }
}

unsafe fn conn_tcp_process(ci: usize, seq: u32, flags: u8, data: &[u8]) {
    unsafe {
        let state = CONNS[ci].state;
        if state == CONN_SYN_SENT && flags & 0x12 == 0x12 {
            CONNS[ci].recv_ack = seq.wrapping_add(1);
            CONNS[ci].send_seq = CONNS[ci].send_seq.wrapping_add(1);
            CONNS[ci].state = CONN_ESTABLISHED;
            CONNS[ci].retry = 0;
            tcp_send_conn_ack(ci);
            return;
        }
        if state != CONN_ESTABLISHED && state != CONN_CLOSE_WAIT {
            return;
        }
        if flags & 0x10 != 0 {
            if !data.is_empty() {
                let n = core::cmp::min(data.len(), CONNS[ci].recv_buf.len());
                CONNS[ci].recv_buf[..n].copy_from_slice(&data[..n]);
                CONNS[ci].recv_len = n;
                CONNS[ci].recv_ack = seq.wrapping_add(data.len() as u32);
                tcp_send_conn_ack(ci);
            } else {
                CONNS[ci].recv_ack = seq;
            }
            conn_cwnd_update(ci);
        }
        if flags & 0x01 != 0 {
            CONNS[ci].recv_ack = seq.wrapping_add(1);
            CONNS[ci].state = CONN_CLOSE_WAIT;
            tcp_send_conn_ack(ci);
        } else if flags & 0x04 != 0 {
            conn_free(ci);
        }
    }
}

unsafe fn tcp_send_conn_ack(ci: usize) {
    unsafe {
        let c = &CONNS[ci];
        let mut seg = [0u8; 20];
        seg[0] = (c.local_port >> 8) as u8;
        seg[1] = c.local_port as u8;
        seg[2] = (c.remote_port >> 8) as u8;
        seg[3] = c.remote_port as u8;
        seg[4..8].copy_from_slice(&c.send_seq.to_be_bytes());
        seg[8..12].copy_from_slice(&c.recv_ack.to_be_bytes());
        seg[12] = 0x50;
        seg[13] = 0x10;
        let chk = csum_tcp_to(&c.remote, &seg);
        seg[16] = (chk >> 8) as u8;
        seg[17] = chk as u8;
        ipv4_send(&c.remote, 6, &seg);
    }
}

fn conn_cwnd_update(ci: usize) {
    unsafe {
        let c = &mut CONNS[ci];
        if c.cwnd < c.ssthresh {
            c.cwnd = c.cwnd.saturating_add(1);
        } else {
            c.cwnd = c.cwnd.saturating_add(1).min(c.cwnd + 1);
            if c.cwnd == 0 {
                c.cwnd = 1;
            }
        }
    }
}

fn conn_congestion_event(ci: usize) {
    unsafe {
        let c = &mut CONNS[ci];
        let half = (c.cwnd / 2).max(1);
        c.ssthresh = half;
        c.cwnd = 1;
    }
}

fn conn_retransmit(ci: usize) {
    unsafe {
        let state = CONNS[ci].state;
        if state == CONN_SYN_SENT {
            if CONNS[ci].retry < 3 {
                CONNS[ci].retry += 1;
                CONNS[ci].syn_tick = crate::intr::Apic::tick();
                tcp_send_conn_syn(ci);
            } else {
                conn_free(ci);
            }
        } else if state == CONN_ESTABLISHED || state == CONN_CLOSE_WAIT {
            conn_congestion_event(ci);
            CONNS[ci].syn_tick = crate::intr::Apic::tick();
        }
    }
}

unsafe fn tcp_handle(src: &[u8], tcp: &[u8]) {
    if tcp.len() < 20 {
        return;
    }
    let src_port = u16::from_be_bytes([tcp[0], tcp[1]]);
    let dst_port = u16::from_be_bytes([tcp[2], tcp[3]]);
    let seq = u32::from_be_bytes([tcp[4], tcp[5], tcp[6], tcp[7]]);
    let hlen = ((tcp[12] >> 4) as usize) * 4;
    if hlen < 20 || hlen > tcp.len() {
        return;
    }
    let flags = tcp[13];
    let data = &tcp[hlen..];

    let remote4 = [src[0], src[1], src[2], src[3]];
    if let Some(ci) = conn_find(&remote4, src_port, dst_port) {
        conn_tcp_process(ci, seq, flags, data);
        return;
    }
    if flags & 0x02 != 0 {
        let mut listen_ci: Option<usize> = None;
        unsafe {
            for i in 0..MAX_CONNS {
                if CONNS[i].state == CONN_LISTEN && CONNS[i].local_port == dst_port {
                    listen_ci = Some(i);
                    break;
                }
            }
        }
        if let Some(ci) = conn_alloc() {
            unsafe {
                let c = &mut CONNS[ci];
                c.state = CONN_ESTABLISHED;
                c.dir = 1;
                c.local_port = dst_port;
                c.remote = remote4;
                c.remote_port = src_port;
                c.send_seq = 0x4000;
                c.recv_ack = seq.wrapping_add(1);
                c.cwnd = 1;
                c.ssthresh = 64;
            }
            tcp_send_conn_synack(ci, seq.wrapping_add(1));
            return;
        }
        let _ = listen_ci;
    }

    if TCP_STATE == 1 && dst_port == TCP_LOCAL_PORT && src == TCP_CONNECT_IP {
        if flags & 0x12 == 0x12 {
            TCP_RECV_ACK = seq.wrapping_add(1);
            TCP_SEND_SEQ = TCP_SEND_SEQ.wrapping_add(1);
            let mut seg = [0u8; 20];
            seg[0] = (TCP_LOCAL_PORT >> 8) as u8;
            seg[1] = TCP_LOCAL_PORT as u8;
            seg[2] = (TCP_CONNECT_PORT >> 8) as u8;
            seg[3] = TCP_CONNECT_PORT as u8;
            seg[4..8].copy_from_slice(&TCP_SEND_SEQ.to_be_bytes());
            seg[8..12].copy_from_slice(&TCP_RECV_ACK.to_be_bytes());
            seg[12] = 0x50;
            seg[13] = 0x10;
            let c = csum_tcp_to(&TCP_CONNECT_IP, &seg);
            seg[16] = (c >> 8) as u8;
            seg[17] = c as u8;
            ipv4_send(&TCP_CONNECT_IP, 6, &seg);
            TCP_STATE = 2;
        }
        return;
    }
    if TCP_STATE == 2 && dst_port == TCP_LOCAL_PORT && src == TCP_CONNECT_IP {
        if flags & 0x10 != 0 && !data.is_empty() {
            let n = core::cmp::min(data.len(), TCP_RECV_BUF.len());
            TCP_RECV_BUF[..n].copy_from_slice(&data[..n]);
            TCP_RECV_LEN = n;
            TCP_RECV_ACK = seq.wrapping_add(data.len() as u32);
            let mut seg = [0u8; 20];
            seg[0] = (TCP_LOCAL_PORT >> 8) as u8;
            seg[1] = TCP_LOCAL_PORT as u8;
            seg[2] = (TCP_CONNECT_PORT >> 8) as u8;
            seg[3] = TCP_CONNECT_PORT as u8;
            seg[4..8].copy_from_slice(&TCP_SEND_SEQ.to_be_bytes());
            seg[8..12].copy_from_slice(&TCP_RECV_ACK.to_be_bytes());
            seg[12] = 0x50;
            seg[13] = 0x10;
            let c = csum_tcp_to(&TCP_CONNECT_IP, &seg);
            seg[16] = (c >> 8) as u8;
            seg[17] = c as u8;
            ipv4_send(&TCP_CONNECT_IP, 6, &seg);
        } else if flags & 0x10 != 0 {
            TCP_RECV_ACK = seq;
        }
        if flags & 0x01 != 0 {
            TCP_RECV_ACK = seq.wrapping_add(1);
            let mut seg = [0u8; 20];
            seg[0] = (TCP_LOCAL_PORT >> 8) as u8;
            seg[1] = TCP_LOCAL_PORT as u8;
            seg[2] = (TCP_CONNECT_PORT >> 8) as u8;
            seg[3] = TCP_CONNECT_PORT as u8;
            seg[4..8].copy_from_slice(&TCP_SEND_SEQ.to_be_bytes());
            seg[8..12].copy_from_slice(&TCP_RECV_ACK.to_be_bytes());
            seg[12] = 0x50;
            seg[13] = 0x10;
            let c = csum_tcp_to(&TCP_CONNECT_IP, &seg);
            seg[16] = (c >> 8) as u8;
            seg[17] = c as u8;
            ipv4_send(&TCP_CONNECT_IP, 6, &seg);
            TCP_STATE = 0;
        } else if flags & 0x04 != 0 {
            TCP_STATE = 0;
        }
        return;
    }

    if dst_port != TCP_PORT {
        return;
    }
    if flags & 0x02 != 0 && !TCP_ESTABLISHED {
        TCP_REMOTE = [src[0], src[1], src[2], src[3]];
        TCP_REMOTE_PORT = src_port;
        TCP_ACK = seq.wrapping_add(1);
        tcp_send(true, false, &[]);
    } else if TCP_ESTABLISHED && flags & 0x10 != 0 && !data.is_empty() {
        TCP_ACK = seq.wrapping_add(data.len() as u32);
        tcp_send(false, false, data);
    } else if flags & 0x10 != 0 && TCP_ESTABLISHED {
        TCP_ACK = seq;
    }
    if flags & 0x04 != 0 {
        TCP_ESTABLISHED = false;
    }
}

unsafe fn tcp_send(syn: bool, _fin: bool, data: &[u8]) {
    let hlen = 20;
    let mut seg = [0u8; 1500];
    seg[0] = (src_port_tcp() >> 8) as u8;
    seg[1] = src_port_tcp() as u8;
    seg[2] = (TCP_PORT >> 8) as u8;
    seg[3] = TCP_PORT as u8;
    seg[4..8].copy_from_slice(&TCP_LOCAL_SEQ.to_be_bytes());
    seg[8..12].copy_from_slice(&TCP_ACK.to_be_bytes());
    seg[12] = 0x50;
    let mut f = 0x10u8;
    if syn {
        f |= 0x02;
    }
    seg[13] = f;
    let total = hlen + data.len();
    seg[hlen..total].copy_from_slice(data);
    let c = csum_tcp(&seg[..total]);
    seg[16] = (c >> 8) as u8;
    seg[17] = c as u8;
    if syn {
        TCP_LOCAL_SEQ = TCP_LOCAL_SEQ.wrapping_add(1);
        TCP_ESTABLISHED = true;
    } else if !data.is_empty() {
        TCP_LOCAL_SEQ = TCP_LOCAL_SEQ.wrapping_add(data.len() as u32);
    }
    ipv4_send(&TCP_REMOTE, 6, &seg[..total]);
}

fn src_port_tcp() -> u16 {
    8080
}

unsafe fn csum_tcp(seg: &[u8]) -> u16 {
    csum_tcp_to(&TCP_REMOTE, seg)
}

unsafe fn csum_tcp_to(dst: &[u8], seg: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for b in dst {
        sum += (*b as u32) << 8;
    }
    for b in &MY_IP {
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

const MY_IP6: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
static mut IPV6_GW: [u8; 16] = [0; 16];
static mut IPV6_GW_VALID: bool = false;
static mut IPV6_DAD_SENT: bool = false;

pub fn ipv6_gw() -> [u8; 16] {
    unsafe { IPV6_GW }
}

pub fn ipv6_gw_valid() -> bool {
    unsafe { IPV6_GW_VALID }
}

unsafe fn ipv6_handle(pkt: &[u8]) {
    if pkt.len() < 40 {
        return;
    }
    let next = pkt[6];
    if next == 58 {
        let icmp = &pkt[40..];
        if icmp.len() >= 8 && icmp[0] == 128 {
            let src = &pkt[8..24];
            let mut reply = [0u8; 64];
            let n = core::cmp::min(icmp.len(), 64);
            reply[..n].copy_from_slice(&icmp[..n]);
            reply[0] = 129;
            reply[2] = 0;
            reply[3] = 0;
            let mut sum: u32 = 0;
            for b in src {
                sum += (*b as u32) << 8;
            }
            for b in &MY_IP6 {
                sum += *b as u32;
            }
            sum += (n as u32) << 8;
            sum += 58;
            let mut i = 0;
            while i + 1 < n {
                sum += ((reply[i] as u32) << 8) | reply[i + 1] as u32;
                i += 2;
            }
            if i < n {
                sum += (reply[i] as u32) << 8;
            }
            while sum >> 16 != 0 {
                sum = (sum & 0xFFFF) + (sum >> 16);
            }
            let c = !(sum as u16);
            reply[2] = (c >> 8) as u8;
            reply[3] = c as u8;
            ipv6_send(src, &reply[..n]);
        } else if icmp.len() >= 24 && icmp[0] == 135 {
            let target = &icmp[8..24];
            if target == MY_IP6 {
                let mut na = [0u8; 24];
                na[0] = 136;
                na[1] = 0;
                na[2] = 0;
                na[3] = 0;
                na[4] = 0x40;
                na[8..24].copy_from_slice(&MY_IP6);
                let mut sum: u32 = 0;
                for b in &pkt[8..24] {
                    sum += (*b as u32) << 8;
                }
                for b in &MY_IP6 {
                    sum += *b as u32;
                }
                sum += 24u32 << 8;
                sum += 58;
                let mut i = 0;
                while i + 1 < 24 {
                    sum += ((na[i] as u32) << 8) | na[i + 1] as u32;
                    i += 2;
                }
                while sum >> 16 != 0 {
                    sum = (sum & 0xFFFF) + (sum >> 16);
                }
                let c = !(sum as u16);
                na[2] = (c >> 8) as u8;
                na[3] = c as u8;
                let mut opt = [0u8; 8];
                opt[0] = 2;
                opt[1] = 1;
                opt[2..8].copy_from_slice(&MAC);
                let mut na_pkt = [0u8; 32];
                na_pkt[..24].copy_from_slice(&na);
                na_pkt[24..32].copy_from_slice(&opt);
                ipv6_send(&pkt[8..24], &na_pkt);
            }
        } else if icmp.len() >= 16 && icmp[0] == 134 {
            IPV6_GW.copy_from_slice(&pkt[8..24]);
            IPV6_GW_VALID = true;
            crate::println!(
                "gvinter: ipv6 ra gw={:x}:{:x}",
                pkt[8], pkt[9]
            );
        }
    }
}

unsafe fn ipv6_send(dst: &[u8], payload: &[u8]) {
    let mut pkt = [0u8; 1500];
    pkt[0] = 0x60;
    let total = 40 + payload.len();
    pkt[4] = (total >> 8) as u8;
    pkt[5] = total as u8;
    pkt[6] = 58;
    pkt[7] = 64;
    pkt[8..24].copy_from_slice(&MY_IP6);
    pkt[24..40].copy_from_slice(dst);
    pkt[40..total].copy_from_slice(payload);
    eth_send(&BCAST, 0x86DD, &pkt[..total]);
}

#[allow(dead_code)]
pub fn wifi_connect(_ssid: &[u8], _key: &[u8]) -> u32 {
    1
}
#[allow(dead_code)]
pub fn wifi_status() -> u32 {
    0
}
#[allow(dead_code)]
pub fn wifi_disconnect() -> u32 {
    1
}

static mut CAPR: u16 = 0;

static mut RAW_OPEN: bool = false;
static mut RAW_BUF: [u8; 2048] = [0; 2048];
static mut RAW_LEN: usize = 0;

pub fn raw_open() -> u32 {
    unsafe {
        if IO_BASE == 0 {
            return 1;
        }
        RAW_OPEN = true;
        RAW_LEN = 0;
    }
    0
}

pub fn raw_close() {
    unsafe {
        RAW_OPEN = false;
        RAW_LEN = 0;
    }
}

pub fn raw_send(data: &[u8]) -> u32 {
    unsafe {
        if IO_BASE == 0 || !LINK_UP || data.len() > 1500 {
            return 1;
        }
        send_frame(data);
    }
    0
}

pub fn raw_recv(out: &mut [u8]) -> usize {
    unsafe {
        let n = core::cmp::min(RAW_LEN, out.len());
        out[..n].copy_from_slice(&RAW_BUF[..n]);
        RAW_LEN = 0;
        n
    }
}

pub fn poll() {
    unsafe {
        if IO_BASE == 0 {
            return;
        }
        let now = crate::intr::Apic::tick();
        for i in 0..MAX_CONNS {
            let st = CONNS[i].state;
            if (st == CONN_SYN_SENT || st == CONN_ESTABLISHED || st == CONN_CLOSE_WAIT)
                && now.wrapping_sub(CONNS[i].syn_tick) > 1000
            {
                conn_retransmit(i);
            }
        }
        if TCP_STATE == 1 && TCP_RETRY < 3 {
            if now.wrapping_sub(TCP_SYN_TICK) > 1000 {
                TCP_RETRY += 1;
                TCP_SYN_TICK = now;
                tcp_send_syn();
            }
        }
        let cbr = inw(IO_BASE + REG_CBR);
        if cbr == CAPR {
            return;
        }
        let cap = CAPR as usize;
        let hdr = u32::from_le_bytes([
            RX_BUF_MEM.add(cap).read_volatile(),
            RX_BUF_MEM.add(cap + 1).read_volatile(),
            RX_BUF_MEM.add(cap + 2).read_volatile(),
            RX_BUF_MEM.add(cap + 3).read_volatile(),
        ]);
        let len = (hdr & 0x1FFF) as usize;
        if hdr & 0x8000 != 0 && len > 0 && len <= 1518 {
            let data = core::slice::from_raw_parts(RX_BUF_MEM.add(cap + 4), len);
            if len >= 14 {
                if RAW_OPEN {
                    let n = core::cmp::min(len, RAW_BUF.len());
                    RAW_BUF[..n].copy_from_slice(&data[..n]);
                    RAW_LEN = n;
                }
                let et = u16::from_be_bytes([data[12], data[13]]);
                let payload = &data[14..];
                crate::println!("gvinter: rx ethtype={:#06x} len={}", et, len);
                match et {
                    0x0806 => arp_handle(payload),
                    0x0800 => ipv4_handle(payload),
                    0x86DD => ipv6_handle(payload),
                    _ => {}
                }
            }
        }
        let used = (4 + len + 3) & !3;
        CAPR = ((CAPR as usize + used) % RX_BUF_SIZE) as u16;
        outw(IO_BASE + REG_CAPR, CAPR);
    }
}

unsafe fn udp_send(dst: &[u8], sport: u16, dport: u16, payload: &[u8]) {
    let mut seg = [0u8; 1500];
    let n = 8 + payload.len();
    seg[0] = (sport >> 8) as u8;
    seg[1] = sport as u8;
    seg[2] = (dport >> 8) as u8;
    seg[3] = dport as u8;
    seg[4] = (n >> 8) as u8;
    seg[5] = n as u8;
    seg[8..n].copy_from_slice(payload);
    let mut sum: u32 = 0;
    for b in &MY_IP {
        sum += *b as u32;
    }
    for b in dst {
        sum += (*b as u32) << 8;
    }
    sum += 17;
    sum += n as u32;
    let mut i = 0;
    while i + 1 < n {
        sum += ((seg[i] as u32) << 8) | seg[i + 1] as u32;
        i += 2;
    }
    if i < n {
        sum += (seg[i] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    let c = !(sum as u16);
    seg[6] = (c >> 8) as u8;
    seg[7] = c as u8;
    ipv4_send(dst, 17, &seg[..n]);
}

pub fn dhcp_start() -> u32 {
    unsafe {
        if IO_BASE == 0 || !LINK_UP {
            return 1;
        }
        DHCP_XID = DHCP_XID.wrapping_add(1);
        DHCP_STATE = 1;
        DHCP_DONE = false;
        dhcp_send_discover();
    }
    0
}

unsafe fn dhcp_send_discover() {
    let mut pkt = [0u8; 300];
    pkt[0] = 1;
    pkt[1] = 1;
    pkt[2] = 6;
    pkt[4..8].copy_from_slice(&DHCP_XID.to_be_bytes());
    pkt[10] = 0x80;
    pkt[11] = 0x00;
    pkt[28..34].copy_from_slice(&MAC);
    pkt[236..240].copy_from_slice(&[0x63, 0x82, 0x53, 0x63]);
    let mut o = 240;
    pkt[o] = 53;
    pkt[o + 1] = 1;
    pkt[o + 2] = 1;
    o += 3;
    pkt[o] = 55;
    pkt[o + 1] = 3;
    pkt[o + 2] = 1;
    pkt[o + 3] = 3;
    pkt[o + 4] = 6;
    o += 5;
    pkt[o] = 255;
    o += 1;
    let bcast = [255u8; 4];
    udp_send(&bcast, DHCP_CLIENT_PORT, DHCP_SERVER_PORT, &pkt[..o]);
}

unsafe fn dhcp_handle(payload: &[u8]) {
    if payload.len() < 240 || payload[0] != 2 {
        return;
    }
    if u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]) != DHCP_XID {
        return;
    }
    let yiaddr = [payload[16], payload[17], payload[18], payload[19]];
    if yiaddr == [0; 4] {
        return;
    }
    let mut msg_type = 0u8;
    let mut server = [0u8; 4];
    let mut o = 240usize;
    while o + 2 <= payload.len() {
        let t = payload[o];
        if t == 255 {
            break;
        }
        let l = payload[o + 1] as usize;
        if t == 0 {
            o += 1;
            continue;
        }
        if o + 2 + l > payload.len() {
            break;
        }
        if t == 53 && l >= 1 {
            msg_type = payload[o + 2];
        } else if t == 54 && l >= 4 {
            server.copy_from_slice(&payload[o + 2..o + 6]);
        }
        o += 2 + l;
    }
    if DHCP_STATE == 1 && msg_type == 2 {
        DHCP_OFFER_IP = yiaddr;
        DHCP_SERVER = server;
        DHCP_STATE = 2;
        dhcp_send_request();
    } else if DHCP_STATE == 2 && msg_type == 5 {
        MY_IP = yiaddr;
        DHCP_DONE = true;
        DHCP_STATE = 0;
        crate::println!(
            "gvinter: dhcp ack ip={}.{}.{}.{}",
            MY_IP[0], MY_IP[1], MY_IP[2], MY_IP[3]
        );
    }
}

unsafe fn dhcp_send_request() {
    let mut pkt = [0u8; 300];
    pkt[0] = 1;
    pkt[1] = 1;
    pkt[2] = 6;
    pkt[4..8].copy_from_slice(&DHCP_XID.to_be_bytes());
    pkt[10] = 0x80;
    pkt[11] = 0x00;
    pkt[28..34].copy_from_slice(&MAC);
    pkt[236..240].copy_from_slice(&[0x63, 0x82, 0x53, 0x63]);
    let mut o = 240;
    pkt[o] = 53;
    pkt[o + 1] = 1;
    pkt[o + 2] = 3;
    o += 3;
    pkt[o] = 50;
    pkt[o + 1] = 4;
    pkt[o + 2..o + 6].copy_from_slice(&DHCP_OFFER_IP);
    o += 6;
    pkt[o] = 54;
    pkt[o + 1] = 4;
    pkt[o + 2..o + 6].copy_from_slice(&DHCP_SERVER);
    o += 6;
    pkt[o] = 255;
    o += 1;
    let bcast = [255u8; 4];
    udp_send(&bcast, DHCP_CLIENT_PORT, DHCP_SERVER_PORT, &pkt[..o]);
}

pub fn dhcp_done() -> bool {
    unsafe { DHCP_DONE }
}

pub fn dhcp_ip() -> [u8; 4] {
    unsafe { MY_IP }
}

unsafe fn udp_handle(src: &[u8], udp: &[u8]) {
    if udp.len() < 8 {
        return;
    }
    let sport = u16::from_be_bytes([udp[0], udp[1]]);
    let dport = u16::from_be_bytes([udp[2], udp[3]]);
    let payload = &udp[8..];
    if dport == 40001 && DNS_PENDING {
        dns_parse(payload);
    }
    if dport == DHCP_CLIENT_PORT && DHCP_STATE != 0 {
        dhcp_handle(payload);
    }
    if UDP_BOUND_PORT != 0 && dport == UDP_BOUND_PORT {
        let n = core::cmp::min(payload.len(), UDP_RECV_BUF.len());
        UDP_RECV_BUF[..n].copy_from_slice(&payload[..n]);
        UDP_RECV_LEN = n;
        UDP_RECV_PORT = sport;
        let _ = src;
    }
}

pub fn udp_bind(port: u16) -> u32 {
    unsafe {
        if IO_BASE == 0 {
            return 1;
        }
        UDP_BOUND_PORT = port;
        UDP_RECV_LEN = 0;
    }
    0
}

pub fn udp_send_to(dst: &[u8; 4], sport: u16, dport: u16, data: &[u8]) -> u32 {
    unsafe {
        if IO_BASE == 0 || !LINK_UP || data.len() > 1450 {
            return 1;
        }
        udp_send(dst, sport, dport, data);
    }
    0
}

pub fn udp_recv(out: &mut [u8]) -> usize {
    unsafe {
        let n = core::cmp::min(UDP_RECV_LEN, out.len());
        out[..n].copy_from_slice(&UDP_RECV_BUF[..n]);
        UDP_RECV_LEN = 0;
        n
    }
}

pub fn udp_recv_from(out: &mut [u8]) -> (usize, u16) {
    unsafe {
        let n = core::cmp::min(UDP_RECV_LEN, out.len());
        out[..n].copy_from_slice(&UDP_RECV_BUF[..n]);
        let port = UDP_RECV_PORT;
        UDP_RECV_LEN = 0;
        (n, port)
    }
}

pub fn dns_query(name: &[u8]) -> u32 {
    unsafe {
        if IO_BASE == 0 || name.len() == 0 || name.len() > 253 {
            return 1;
        }
        DNS_ID = DNS_ID.wrapping_add(1);
        DNS_PENDING = true;
        DNS_DONE = false;
        let mut q = [0u8; 512];
        let mut n = 12;
        q[0] = (DNS_ID >> 8) as u8;
        q[1] = DNS_ID as u8;
        q[5] = 1;
        q[7] = 1;
        let mut start = 0usize;
        for (i, &c) in name.iter().enumerate() {
            if c == b'.' {
                let part = &name[start..i];
                if part.len() == 0 || part.len() > 63 || n + 1 + part.len() > 500 {
                    return 1;
                }
                q[n] = part.len() as u8;
                n += 1;
                q[n..n + part.len()].copy_from_slice(part);
                n += part.len();
                start = i + 1;
            }
        }
        let part = &name[start..];
        if part.len() == 0 || part.len() > 63 || n + 1 + part.len() > 500 {
            return 1;
        }
        q[n] = part.len() as u8;
        n += 1;
        q[n..n + part.len()].copy_from_slice(part);
        n += part.len();
        q[n] = 0;
        n += 1;
        q[n] = 0;
        q[n + 1] = 1;
        n += 2;
        q[n] = 0;
        q[n + 1] = 1;
        n += 2;
        udp_send(&DNS_SERVER, 40001, DNS_PORT, &q[..n]);
    }
    0
}

unsafe fn dns_parse(payload: &[u8]) {
    DNS_PENDING = false;
    if payload.len() < 12 {
        return;
    }
    let ancount = ((payload[6] as u16) << 8) | payload[7] as u16;
    if ancount == 0 {
        return;
    }
    let mut off = 12usize;
    while off < payload.len() && payload[off] != 0 {
        off += 1 + payload[off] as usize;
    }
    off += 5;
    if off + 16 > payload.len() {
        return;
    }
    let rdlen = ((payload[off + 10] as usize) << 8) | payload[off + 11] as usize;
    let rdata = off + 12;
    if rdlen == 4 && rdata + 4 <= payload.len() {
        DNS_RESULT.copy_from_slice(&payload[rdata..rdata + 4]);
        DNS_DONE = true;
    }
}

pub fn tcp_connect(ip: &[u8; 4], port: u16) -> u32 {
    unsafe {
        if IO_BASE == 0 || !LINK_UP {
            return 1;
        }
        let ci = match conn_alloc() {
            Some(x) => x,
            None => return 1,
        };
        unsafe {
            let c = &mut CONNS[ci];
            c.state = CONN_SYN_SENT;
            c.dir = 0;
            c.local_port = TCP_LOCAL_PORT;
            c.remote = *ip;
            c.remote_port = port;
            c.send_seq = 0x3000;
            c.recv_ack = 0;
            c.recv_len = 0;
            c.retry = 0;
            c.cwnd = 1;
            c.ssthresh = 64;
            tcp_send_conn_syn(ci);
            c.syn_tick = crate::intr::Apic::tick();
        }
        TCP_STATE = 1;
        TCP_CONNECT_IP = *ip;
        TCP_CONNECT_PORT = port;
        TCP_SEND_SEQ = 0x3000;
        TCP_RECV_ACK = 0;
        TCP_RECV_LEN = 0;
        TCP_RETRY = 0;
        tcp_send_syn();
        TCP_SYN_TICK = crate::intr::Apic::tick();
    }
    0
}

unsafe fn tcp_send_conn_syn(ci: usize) {
    unsafe {
        let c = &CONNS[ci];
        let mut seg = [0u8; 20];
        seg[0] = (c.local_port >> 8) as u8;
        seg[1] = c.local_port as u8;
        seg[2] = (c.remote_port >> 8) as u8;
        seg[3] = c.remote_port as u8;
        seg[4..8].copy_from_slice(&c.send_seq.to_be_bytes());
        seg[12] = 0x50;
        seg[13] = 0x02;
        let chk = csum_tcp_to(&c.remote, &seg);
        seg[16] = (chk >> 8) as u8;
        seg[17] = chk as u8;
        ipv4_send(&c.remote, 6, &seg);
    }
}

unsafe fn tcp_send_syn() {
    let mut seg = [0u8; 20];
    seg[0] = (TCP_LOCAL_PORT >> 8) as u8;
    seg[1] = TCP_LOCAL_PORT as u8;
    seg[2] = (TCP_CONNECT_PORT >> 8) as u8;
    seg[3] = TCP_CONNECT_PORT as u8;
    seg[4..8].copy_from_slice(&TCP_SEND_SEQ.to_be_bytes());
    seg[12] = 0x50;
    seg[13] = 0x02;
    let c = csum_tcp_to(&TCP_CONNECT_IP, &seg);
    seg[16] = (c >> 8) as u8;
    seg[17] = c as u8;
    ipv4_send(&TCP_CONNECT_IP, 6, &seg);
}

unsafe fn tcp_send_conn(data: &[u8]) {
    let mut seg = [0u8; 1500];
    let n = 20 + data.len();
    seg[0] = (TCP_LOCAL_PORT >> 8) as u8;
    seg[1] = TCP_LOCAL_PORT as u8;
    seg[2] = (TCP_CONNECT_PORT >> 8) as u8;
    seg[3] = TCP_CONNECT_PORT as u8;
    seg[4..8].copy_from_slice(&TCP_SEND_SEQ.to_be_bytes());
    seg[8..12].copy_from_slice(&TCP_RECV_ACK.to_be_bytes());
    seg[12] = 0x50;
    seg[13] = 0x18;
    seg[20..n].copy_from_slice(data);
    let c = csum_tcp_to(&TCP_CONNECT_IP, &seg[..n]);
    seg[16] = (c >> 8) as u8;
    seg[17] = c as u8;
    TCP_SEND_SEQ = TCP_SEND_SEQ.wrapping_add(data.len() as u32);
    ipv4_send(&TCP_CONNECT_IP, 6, &seg[..n]);
}

unsafe fn tcp_close_conn() {
    if TCP_STATE == 2 {
        let mut seg = [0u8; 20];
        seg[0] = (TCP_LOCAL_PORT >> 8) as u8;
        seg[1] = TCP_LOCAL_PORT as u8;
        seg[2] = (TCP_CONNECT_PORT >> 8) as u8;
        seg[3] = TCP_CONNECT_PORT as u8;
        seg[4..8].copy_from_slice(&TCP_SEND_SEQ.to_be_bytes());
        seg[8..12].copy_from_slice(&TCP_RECV_ACK.to_be_bytes());
        seg[12] = 0x50;
        seg[13] = 0x11;
        let c = csum_tcp_to(&TCP_CONNECT_IP, &seg);
        seg[16] = (c >> 8) as u8;
        seg[17] = c as u8;
        ipv4_send(&TCP_CONNECT_IP, 6, &seg);
    }
    TCP_STATE = 0;
}

pub fn http_get(host: &[u8], path: &[u8]) -> u32 {
    unsafe {
        if IO_BASE == 0 {
            return 1;
        }
        if dns_query(host) != 0 {
            return 1;
        }
        let mut t = 0;
        while !DNS_DONE && t < 1000000 {
            poll();
            t += 1;
            core::hint::spin_loop();
        }
        if !DNS_DONE {
            return 1;
        }
        if tcp_connect(&DNS_RESULT, 80) != 0 {
            return 1;
        }
        let mut t2 = 0;
        while TCP_STATE != 2 && t2 < 1000000 {
            poll();
            t2 += 1;
            core::hint::spin_loop();
        }
        if TCP_STATE != 2 {
            return 1;
        }
        let mut req = [0u8; 1024];
        let mut n = 0;
        req[n..n + 4].copy_from_slice(b"GET ");
        n += 4;
        req[n..n + path.len()].copy_from_slice(path);
        n += path.len();
        req[n..n + 11].copy_from_slice(b" HTTP/1.0\r\n");
        n += 11;
        req[n..n + 12].copy_from_slice(b"Host: ");
        n += 12;
        req[n..n + host.len()].copy_from_slice(host);
        n += host.len();
        req[n..n + 4].copy_from_slice(b"\r\n\r\n");
        n += 4;
        tcp_send_conn(&req[..n]);
        let mut t3 = 0;
        while TCP_RECV_LEN == 0 && t3 < 1000000 {
            poll();
            t3 += 1;
            core::hint::spin_loop();
        }
        HTTP_LEN = TCP_RECV_LEN;
        HTTP_BUF[..TCP_RECV_LEN].copy_from_slice(&TCP_RECV_BUF[..TCP_RECV_LEN]);
        tcp_close_conn();
    }
    0
}

pub fn ping(ip: &[u8; 4]) -> u32 {
    unsafe {
        if IO_BASE == 0 {
            return 1;
        }
        let mut iph = [0u8; 20];
        iph[0] = 0x45;
        iph[2] = 0;
        iph[3] = 28;
        iph[8] = 64;
        iph[9] = 1;
        iph[12..16].copy_from_slice(&MY_IP);
        iph[16..20].copy_from_slice(ip);
        let c = csum(&iph);
        iph[10] = (c >> 8) as u8;
        iph[11] = c as u8;
        let mut icmp = [0u8; 8];
        icmp[0] = 8;
        icmp[4] = 1;
        let c2 = csum(&icmp);
        icmp[2] = (c2 >> 8) as u8;
        icmp[3] = c2 as u8;
        let mut pkt = [0u8; 28];
        pkt[..20].copy_from_slice(&iph);
        pkt[20..28].copy_from_slice(&icmp);
        PING_REPLY = false;
        eth_send(&BCAST, 0x0800, &pkt);
        let start = crate::intr::Apic::tick();
        while crate::intr::Apic::tick().wrapping_sub(start) < 2000 && !PING_REPLY {
            poll();
            core::hint::spin_loop();
        }
        if PING_REPLY {
            0
        } else {
            1
        }
    }
}

pub fn dns_done() -> bool {
    unsafe { DNS_DONE }
}

pub fn dns_result() -> [u8; 4] {
    unsafe { DNS_RESULT }
}

pub fn http_data_len() -> usize {
    unsafe { HTTP_LEN }
}

pub fn http_data_ptr() -> *const u8 {
    unsafe { HTTP_BUF.as_ptr() }
}

pub fn net_state() -> u32 {
    unsafe { TCP_STATE as u32 }
}

pub fn net_test() {
    unsafe {
        if IO_BASE == 0 {
            return;
        }
        let mut arp = [0u8; 28];
        arp[0] = 0;
        arp[1] = 1;
        arp[2] = 0x08;
        arp[3] = 0x00;
        arp[4] = 6;
        arp[5] = 4;
        arp[6] = 0;
        arp[7] = 1;
        arp[8..14].copy_from_slice(&MAC);
        arp[14..18].copy_from_slice(&MY_IP);
        arp[24..28].copy_from_slice(&GATEWAY_IP);
        eth_send(&BCAST, 0x0806, &arp);
        crate::println!("gvinter: arp request sent");
        crate::println!("gvinter: ping 10.0.2.2 ...");
        let mut ip = [0u8; 20];
        ip[0] = 0x45;
        ip[2] = 0;
        ip[3] = 28;
        ip[8] = 64;
        ip[9] = 1;
        ip[12..16].copy_from_slice(&MY_IP);
        ip[16..20].copy_from_slice(&GATEWAY_IP);
        let c = csum(&ip[..20]);
        ip[10] = (c >> 8) as u8;
        ip[11] = c as u8;
        let mut icmp = [0u8; 8];
        icmp[0] = 8;
        icmp[1] = 0;
        icmp[4] = 1;
        icmp[5] = 0;
        let c = csum(&icmp);
        icmp[2] = (c >> 8) as u8;
        icmp[3] = c as u8;
        let mut pkt = [0u8; 28];
        pkt[..20].copy_from_slice(&ip);
        pkt[20..28].copy_from_slice(&icmp);
        eth_send(&BCAST, 0x0800, &pkt);
        crate::println!("gvinter: ping sent, waiting for reply");
    }
}

pub fn taojie_chuangjian() -> u32 {
    unsafe {
        if IO_BASE == 0 {
            return 0xFFFFFFFF;
        }
        match conn_alloc() {
            Some(ci) => (ci + 1) as u32,
            None => 0xFFFFFFFF,
        }
    }
}

pub fn taojie_bangding(fd: u32, port: u16) -> u32 {
    unsafe {
        let ci = fd as usize;
        if ci == 0 || ci > MAX_CONNS {
            return 1;
        }
        let i = ci - 1;
        if CONNS[i].state != CONN_FREE {
            return 1;
        }
        CONNS[i].local_port = port;
        CONNS[i].state = CONN_LISTEN;
    }
    0
}

pub fn taojie_jianting(fd: u32) -> u32 {
    unsafe {
        let ci = fd as usize;
        if ci == 0 || ci > MAX_CONNS {
            return 1;
        }
        let i = ci - 1;
        if CONNS[i].local_port == 0 {
            return 1;
        }
        CONNS[i].state = CONN_LISTEN;
    }
    0
}

pub fn taojie_jieshou(fd: u32) -> u32 {
    unsafe {
        let ci = fd as usize;
        if ci == 0 || ci > MAX_CONNS {
            return 0xFFFFFFFF;
        }
        let li = ci - 1;
        let lport = CONNS[li].local_port;
        for i in 0..MAX_CONNS {
            if CONNS[i].state == CONN_ESTABLISHED
                && CONNS[i].dir == 1
                && CONNS[i].local_port == lport
            {
                return (i + 1) as u32;
            }
        }
    }
    0xFFFFFFFF
}

pub fn taojie_lianjie(fd: u32, ip: &[u8; 4], port: u16) -> u32 {
    unsafe {
        let ci = fd as usize;
        if ci == 0 || ci > MAX_CONNS {
            return 1;
        }
        if IO_BASE == 0 || !LINK_UP {
            return 1;
        }
        let i = ci - 1;
        CONNS[i].state = CONN_SYN_SENT;
        CONNS[i].dir = 0;
        CONNS[i].remote = *ip;
        CONNS[i].remote_port = port;
        CONNS[i].send_seq = 0x3000;
        CONNS[i].recv_ack = 0;
        CONNS[i].recv_len = 0;
        CONNS[i].retry = 0;
        CONNS[i].cwnd = 1;
        CONNS[i].ssthresh = 64;
        tcp_send_conn_syn(i);
        CONNS[i].syn_tick = crate::intr::Apic::tick();
    }
    0
}

pub fn taojie_fasong(fd: u32, data: &[u8]) -> u32 {
    unsafe {
        let ci = fd as usize;
        if ci == 0 || ci > MAX_CONNS {
            return 0;
        }
        let i = ci - 1;
        if CONNS[i].state != CONN_ESTABLISHED && CONNS[i].state != CONN_CLOSE_WAIT {
            return 0;
        }
        tcp_send_conn_data(i, data);
        data.len() as u32
    }
}

unsafe fn tcp_send_conn_data(ci: usize, data: &[u8]) {
    unsafe {
        let c = &CONNS[ci];
        let mut seg = [0u8; 1500];
        let n = 20 + data.len();
        seg[0] = (c.local_port >> 8) as u8;
        seg[1] = c.local_port as u8;
        seg[2] = (c.remote_port >> 8) as u8;
        seg[3] = c.remote_port as u8;
        seg[4..8].copy_from_slice(&c.send_seq.to_be_bytes());
        seg[8..12].copy_from_slice(&c.recv_ack.to_be_bytes());
        seg[12] = 0x50;
        seg[13] = 0x18;
        seg[20..n].copy_from_slice(data);
        let chk = csum_tcp_to(&c.remote, &seg[..n]);
        seg[16] = (chk >> 8) as u8;
        seg[17] = chk as u8;
        CONNS[ci].send_seq = CONNS[ci].send_seq.wrapping_add(data.len() as u32);
        ipv4_send(&c.remote, 6, &seg[..n]);
    }
}

pub fn taojie_duqu(fd: u32, out: &mut [u8]) -> usize {
    unsafe {
        let ci = fd as usize;
        if ci == 0 || ci > MAX_CONNS {
            return 0;
        }
        let i = ci - 1;
        let n = core::cmp::min(CONNS[i].recv_len, out.len());
        out[..n].copy_from_slice(&CONNS[i].recv_buf[..n]);
        CONNS[i].recv_len = 0;
        n
    }
}

pub fn taojie_guanbi(fd: u32) -> u32 {
    unsafe {
        let ci = fd as usize;
        if ci == 0 || ci > MAX_CONNS {
            return 1;
        }
        let i = ci - 1;
        let st = CONNS[i].state;
        if st == CONN_ESTABLISHED || st == CONN_CLOSE_WAIT {
            tcp_send_conn_fin(i);
        }
        conn_free(i);
    }
    0
}

unsafe fn tcp_send_conn_fin(ci: usize) {
    unsafe {
        let c = &CONNS[ci];
        let mut seg = [0u8; 20];
        seg[0] = (c.local_port >> 8) as u8;
        seg[1] = c.local_port as u8;
        seg[2] = (c.remote_port >> 8) as u8;
        seg[3] = c.remote_port as u8;
        seg[4..8].copy_from_slice(&c.send_seq.to_be_bytes());
        seg[8..12].copy_from_slice(&c.recv_ack.to_be_bytes());
        seg[12] = 0x50;
        seg[13] = 0x11;
        let chk = csum_tcp_to(&c.remote, &seg);
        seg[16] = (chk >> 8) as u8;
        seg[17] = chk as u8;
        ipv4_send(&c.remote, 6, &seg);
    }
}

pub fn http_server_start(port: u16) -> u32 {
    unsafe {
        if IO_BASE == 0 {
            return 1;
        }
        HTTP_SERVER_FD = taojie_chuangjian();
        if HTTP_SERVER_FD == 0xFFFFFFFF {
            return 1;
        }
        if taojie_bangding(HTTP_SERVER_FD, port) != 0 {
            return 1;
        }
        crate::println!(
            "gvinter: http server on port {}",
            port
        );
    }
    0
}

static mut HTTP_SERVER_FD: u32 = 0;

pub fn http_server_poll() {
    unsafe {
        if HTTP_SERVER_FD == 0 {
            return;
        }
        let cfd = taojie_jieshou(HTTP_SERVER_FD);
        if cfd == 0xFFFFFFFF {
            return;
        }
        let mut req = [0u8; 1024];
        let n = taojie_duqu(cfd, &mut req);
        if n == 0 {
            taojie_guanbi(cfd);
            return;
        }
        let mut path = [0u8; 128];
        let mut plen = 0usize;
        let mut i = 0usize;
        if n > 4 && req[0] == b'G' && req[1] == b'E' && req[2] == b'T' && req[3] == b' ' {
            i = 4;
            while i < n && req[i] != b' ' && plen < path.len() {
                path[plen] = req[i];
                plen += 1;
                i += 1;
            }
        }
        let mut body = [0u8; 512];
        let mut blen = 0usize;
        let p = if plen > 0 { &path[..plen] } else { b"/" };
        let title = b"Gvtcier HTTP";
        blen += put_str(&mut body, blen, b"<html><head><title>");
        blen += put_str(&mut body, blen, title);
        blen += put_str(&mut body, blen, b"</title></head><body><h1>Gvtcier</h1><p>path: ");
        blen += put_str(&mut body, blen, p);
        blen += put_str(&mut body, blen, b"</p><p>Gvinter HTTP server</p></body></html>");
        let mut resp = [0u8; 2048];
        let mut rl = 0usize;
        rl += put_str(&mut resp, rl, b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: ");
        let mut tmp = [0u8; 12];
        let mut ti = 0usize;
        let mut v = blen;
        loop {
            tmp[ti] = b'0' + (v % 10) as u8;
            ti += 1;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        while ti > 0 {
            ti -= 1;
            resp[rl] = tmp[ti];
            rl += 1;
        }
        rl += put_str(&mut resp, rl, b"\r\n\r\n");
        for k in 0..blen {
            if rl < resp.len() {
                resp[rl] = body[k];
                rl += 1;
            }
        }
        taojie_fasong(cfd, &resp[..rl]);
        taojie_guanbi(cfd);
        crate::println!("gvinter: http served");
    }
}

fn put_str(dst: &mut [u8], mut off: usize, s: &[u8]) -> usize {
    for &b in s {
        if off < dst.len() {
            dst[off] = b;
            off += 1;
        }
    }
    s.len()
}
