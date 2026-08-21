fn main() {
    let dir = std::env::current_dir().unwrap();
    println!("cargo:rustc-link-arg=-T{}", dir.join("link.ld").display());
    println!("cargo:rerun-if-changed=src/trampoline.S");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let status = std::process::Command::new("nasm")
        .args(["-f", "elf64", "src/trampoline.S", "-o"])
        .arg(format!("{}/trampoline.o", out_dir))
        .status()
        .unwrap();
    assert!(status.success(), "nasm failed");
    println!("cargo:rustc-link-arg=-L{}", out_dir);
    println!("cargo:rustc-link-arg=-l:trampoline.o");
}
