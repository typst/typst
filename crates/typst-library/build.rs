use gix_odb::loose::Store as Odb;
use gix_ref::file::{ReferenceExt, Store};
use gix_ref::store::WriteReflog;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // note: the build script starts with its working directory equal to CARGO_MANIFEST_DIR, so crates/typst-library
    let cwd = std::env::current_dir().unwrap();
    let root = cwd.parent().unwrap().parent().unwrap();
    let store = Store::at(root.join(".git"), WriteReflog::Disable, Default::default());
    let odb = Odb::at(root.join(".git").join("objects"), Default::default());
    let mut head = store.find("HEAD")?;
    head.peel_to_id_in_place(&store, |oid, buf| {
        odb.try_find(oid, buf).map(|po| po.map(|data| (data.kind, data.data)))
    })?;
    println!("cargo:rustc-env=TYPST_COMMIT={}", head.peeled.unwrap().to_hex());
    Ok(())
}
