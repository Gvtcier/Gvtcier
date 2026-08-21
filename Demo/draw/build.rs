fn main() {
    let dir = std::env::current_dir().unwrap();
    println!("cargo:rustc-link-arg=-T{}", dir.join("link.ld").display());
}
