use std::fs::{self, create_dir_all, read_dir, read_to_string};
use std::ffi::OsStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    create_dir_all("tests/cache")?;

    // Make sure the script reruns if this file changes or files are
    // added/deleted in the parsing folder.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=tests/parsing");

    // Compile all parser tests into a single giant vector.
    let mut code = "vec![".to_string();

    for entry in read_dir("tests/parsing")? {
        let path = entry?.path();
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }

        let name = path
            .file_stem().ok_or("expected file stem")?
            .to_string_lossy();

        // Make sure this also reruns if the contents of a file in parsing
        // change. This is not ensured by rerunning only on the folder.
        println!("cargo:rerun-if-changed=tests/parsing/{}.rs", name);

        code.push_str(&format!("(\"{}\", tokens!{{", name));

        // Replace the `=>` arrows with a double arrow indicating the line
        // number in the middle, such that the tester can tell which line number
        // a test originated from.
        let file = read_to_string(&path)?;
        for (index, line) in file.lines().enumerate() {
            let line = line.replace("=>", &format!("=>({})=>", index + 1));
            code.push_str(&line);
            code.push('\n');
        }

        code.push_str("}),");
    }

    code.push(']');

    fs::write("tests/cache/parse", code)?;

    Ok(())
}
