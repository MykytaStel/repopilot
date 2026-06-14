// A Cargo build script is a legitimate entrypoint: it is compiled and run by
// Cargo, never imported by another module, so its fan-in is always zero. It
// must not be reported as a dead module.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
