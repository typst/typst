fn main() {
    println!("cargo:rerun-if-env-changed=TYPST_VERSION");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
}
