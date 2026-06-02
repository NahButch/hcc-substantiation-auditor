use engine::placeholder;

fn main() {
    println!("HCC Substantiation Auditor — agent v{}", env!("CARGO_PKG_VERSION"));
    // Verify the engine dependency is wired correctly at startup.
    let _ = placeholder();
}
