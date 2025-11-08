fn main() {
    println!("cargo:rerun-if-env-changed=TYPST_VERSION");

    if option_env!("TYPST_VERSION").is_none() {
        println!("cargo:rustc-env=TYPST_VERSION={}", env!("CARGO_PKG_VERSION"));
    }
}
