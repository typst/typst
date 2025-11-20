fn main() {
    println!("cargo:rerun-if-env-changed=TYPST_VERSION");
}
