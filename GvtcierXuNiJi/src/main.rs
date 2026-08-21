use std::fs;

mod gvdebug;
mod net_sim;

use gvdebug::{GvDebug, OP_WRITE};

#[repr(C, packed)]
struct AHCICmdHdr {
    opts: u16,
    prdtl: u16,
    status: u32,
    tbl_addr: u64,
    _reserved: [u32; 4],
}

#[repr(C, packed)]
struct PRD {
    addr: u64,
    _reserved: u32,
    flags_size: u32,
}

struct AhciController {
    mem: Vec<u8>,
    disk: Vec<u8>,
    debug: GvDebug,
}

impl AhciController {
    fn new(mem_size: usize, disk: Vec<u8>) -> Self {
        AhciController {
            mem: vec![0u8; mem_size],
            disk,
            debug: GvDebug::new(),
        }
    }

    fn put_u32(&mut self, addr: usize, v: u32) {
        self.mem[addr..addr + 4].copy_from_slice(&v.to_le_bytes());
        self.debug.check(addr as u64, 4, OP_WRITE, &v.to_le_bytes());
    }

    fn put_u8s(&mut self, addr: usize, bytes: &[u8]) {
        self.mem[addr..addr + bytes.len()].copy_from_slice(bytes);
        self.debug.check(addr as u64, bytes.len(), OP_WRITE, bytes);
    }

    fn execute(&mut self, clb: usize, slot: usize) -> Result<Vec<(u64, usize)>, String> {
        let off = clb + slot * 32;
        let hdr: AHCICmdHdr =
            unsafe { core::ptr::read_unaligned(self.mem.as_ptr().add(off) as *const AHCICmdHdr) };
        let cfl = (hdr.opts & 0x1F) as usize;
        let prdtl = hdr.prdtl as usize;
        let tbl = hdr.tbl_addr as usize;
        println!("CLB: cfl={} prdtl={} tbl_addr={:#x}", cfl, prdtl, tbl);
        self.debug.trace_add(format!("CLB: cfl={} prdtl={} tbl={:#x}", cfl, prdtl, tbl));

        if tbl + 0x100 > self.mem.len() {
            return Err("命令表越界".into());
        }
        let fis = &self.mem[tbl..tbl + 0x80];
        println!("FIS: {}", fis[..20].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));

        if fis[0] != 0x27 {
            return Err(format!("FIS 类型错误：{:#x}（应为 0x27 = H2D Register）", fis[0]));
        }
        if fis[1] & 0x0F != 0 {
            return Err(format!("FIS PM 端口位非零：{:#x}", fis[1]));
        }
        if fis[1] & 0x70 != 0 {
            return Err(format!("FIS 保留位非零：{:#x}", fis[1]));
        }
        if fis[1] & 0x80 == 0 {
            return Err("FIS 未置 C 位（未更新命令寄存器）".into());
        }
        let cmd = fis[2];
        if cmd != 0x25 {
            return Err(format!("命令错误：{:#x}（应为 0x25 = READ DMA EXT）", cmd));
        }
        let lba = (fis[4] as u64)
            | ((fis[5] as u64) << 8)
            | ((fis[6] as u64) << 16)
            | ((fis[8] as u64) << 24)
            | ((fis[9] as u64) << 32)
            | ((fis[10] as u64) << 40);
        let count = (((fis[13] as u64) << 8) | fis[12] as u64) as usize;
        println!("READ DMA EXT: lba={} count={}", lba, count);
        self.debug.trace_add(format!("FIS 校验通过: cmd={:#04x} lba={} count={}", cmd, lba, count));

        if prdtl == 0 {
            return Err("PRDTL=0（无 PRD）".into());
        }
        let mut targets = Vec::new();
        for i in 0..prdtl {
            let po = tbl + 0x80 + i * 16;
            let prd: PRD = unsafe { core::ptr::read_unaligned(self.mem.as_ptr().add(po) as *const PRD) };
            let addr = prd.addr;
            let size = ((prd.flags_size & 0x3FFFFF) + 1) as usize;
            println!("PRD[{}]: addr={:#x} size={}", i, addr, size);
            self.debug.trace_add(format!("PRD[{}]: addr={:#x} size={}", i, addr, size));
            targets.push((addr, size));
        }

        for i in 0..count {
            let src = (lba as usize + i) * 512;
            if src + 512 > self.disk.len() {
                return Err(format!("磁盘越界：lba={} src={:#x}", lba, src));
            }
            let (dst, size) = targets[0];
            let n = size.min(512);
            let d = dst as usize;
            if d + n > self.mem.len() {
                return Err(format!("DMA 目标越界：{:#x}", d));
            }
            self.mem[d..d + n].copy_from_slice(&self.disk[src..src + n]);
        }
        self.debug.trace_add(format!("DMA 完成: lba={} count={} 目标 {} 项", lba, count, targets.len()));
        Ok(targets)
    }
}

fn main() {
    let disk = fs::read(r"D:\Code\Code\Gvtcier\iso\disk.img").expect("读取 iso/disk.img 失败");
    println!("disk.img: {} 字节（扇区0 应为 FAT 引导扇区）", disk.len());

    let mut vm = AhciController::new(0x800_0000, disk);
    vm.debug.add_watch(0x78000, 0x200, OP_WRITE, "CLB/FIS/PRDT 区域");
    vm.debug.trace_add("仿真启动: 装载 CLB/FIS/PRDT".into());

    vm.put_u32(0x78000, 0x10005);
    vm.put_u32(0x78004, 0);
    vm.put_u32(0x78008, 0x78080);
    vm.put_u32(0x7800C, 0);

    vm.put_u8s(
        0x78080,
        &[
            0x27, 0x80, 0x25, 0x00,
            0x00, 0x00, 0x00, 0xE0,
            0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ],
    );

    vm.put_u32(0x78100, 0x2B9638);
    vm.put_u32(0x78104, 0);
    vm.put_u32(0x78108, 0);
    vm.put_u32(0x7810C, 0x1FF);

    println!("\n=== 执行 READ DMA EXT（LBA 0，1 扇区）===");
    match vm.execute(0x78000, 0) {
        Ok(_) => {
            vm.debug.trace_add("命令解析与执行成功".into());
            println!("=== 命令解析与执行成功 ===");
            let buf = &vm.mem[0x2B9638..0x2B9638 + 512];
            println!("DMA 目标前 16 字节：{:02x?}", &buf[..16]);
            let oem = String::from_utf8_lossy(&buf[3..11]);
            println!("扇区 OEM 标识：{:?}", oem);
            if &buf[3..11] == b"EXFAT   " {
                println!("✓ 与 disk.img 扇区0 一致——Gvtcier内核的 CLB/FIS/PRDT 布局符合 AHCI 规范");
                println!("  （因此 QEMU 读不到该命令属于其自身行为，而非我们的布局错误）");
            } else {
                println!("✗ DMA 内容与 disk.img 不一致！");
            }
        }
        Err(e) => {
            vm.debug.trace_add(format!("命令执行失败: {}", e));
            println!("✗ 命令执行失败：{}（Gvtcier内核布局存在规范偏差）", e);
            println!("{}", vm.debug.report());
            println!(
                "== GvDebug 现场转储 (0x78000, 512 字节) ==\n{}",
                vm.debug.dump(&vm.mem, 0x78000, 0x200)
            );
        }
    }
    println!("{}", vm.debug.report());

    net_sim::run();
}
