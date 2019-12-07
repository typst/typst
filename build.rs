use std::fs;
use std::ffi::OsStr;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=tests/parsing");

    fs::create_dir_all("tests/cache").unwrap();

    let paths = fs::read_dir("tests/parsing").unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension() == Some(OsStr::new("rs")));

    let mut code = "vec![".to_string();
    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let file = fs::read_to_string(&path).unwrap();

        println!("cargo:rerun-if-changed=tests/parsing/{}.rs", name);

        code.push_str(&format!("(\"{}\", tokens!{{", name));

        for (index, line) in file.lines().enumerate() {
            let mut line = line.replace("=>", &format!("=>({})=>", index + 1));
            line.push('\n');
            code.push_str(&line);
        }

        code.push_str("}),");
    }
    code.push(']');

    fs::write("tests/cache/parsing.rs", code).unwrap();
}
