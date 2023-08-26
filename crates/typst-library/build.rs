use git2::Repository;

fn main() {
    // note: the build script starts with its working directory equal to CARGO_MANIFEST_DIR, so crates/typst-library
    let cwd = std::env::current_dir().unwrap();
    let root = cwd.parent().unwrap().parent().unwrap();
    let repo = Repository::init(root).expect("no repo");
    let head = repo.head().expect("no HEAD");
    println!(
        "cargo:rustc-env=TYPST_COMMIT={}",
        head.target().expect("HEAD is not direct")
    );
}
