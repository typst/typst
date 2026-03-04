use std::collections::HashSet;
use std::fmt::{self, Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;
use typst::foundations::Bytes;

#[test]
fn test_help() {
    let output = exec().arg("--help").must_succeed();
    output
        .stdout
        .must_contain("Compiles an input file")
        .must_contain("https://typst.app/docs/tutorial/");
}

#[test]
fn test_compile_pdf() {
    let project = tempfs();
    let title = "Hello from CLI";
    let hello = project.write("hello.typ", format!("#set document(title: \"{title}\")"));
    exec().arg("compile").arg(&hello).must_succeed();
    project.read("hello.pdf").must_start_with("%PDF").must_contain(title);
}

#[test]
fn test_eval() {
    let output = exec().arg("eval").arg("1+2").must_succeed();
    output.stdout.must_match_lines(["3"]);
}

#[test]
fn test_fonts_embedded() {
    let output = exec().arg("fonts").arg("--ignore-system-fonts").must_succeed();
    output.stdout.must_match_lines([
        "DejaVu Sans Mono",
        "Libertinus Serif",
        "New Computer Modern",
        "New Computer Modern Math",
    ]);
}

#[test]
fn test_fonts_path() {
    let fonts = tempfs();
    let mut expected = HashSet::new();
    for (i, data) in typst_dev_assets::fonts().enumerate() {
        let font = typst::text::Font::new(Bytes::new(data), 0).unwrap();
        fonts.write(format!("{i}.ttf"), data);
        expected.insert(font.info().family.clone());
    }
    let output = exec()
        .arg("fonts")
        .arg("--ignore-embedded-fonts")
        .arg("--ignore-system-fonts")
        .arg("--font-path")
        .arg(fonts.path())
        .must_succeed();
    let found = output
        .stdout
        .lines()
        .map(|line| line.to_string())
        .collect::<HashSet<_>>();
    assert_eq!(found, expected);
}

#[test]
fn test_info() {
    let output = exec().arg("info").must_succeed();
    output.stderr.must_start_with("Version");
}

#[test]
fn test_deps() {
    let project = tempfs();
    let main = project.write("main.typ", "#image(\"tiger.jpg\")");
    project.write("tiger.jpg", typst_dev_assets::get_by_name("tiger.jpg").unwrap());
    let output = exec().arg("compile").arg(main).arg("--deps").arg("-").must_succeed();
    output.stdout.must_contain("tiger.jpg").must_contain("main.typ");
}

#[test]
fn test_path_resolved() {
    let project = tempfs();
    let main = project.write("main.typ", "#include \"dir/a.typ\"");
    project.write("dir/a.typ", "#include \"/dir/b.typ\"");
    project.write("dir/b.typ", "#import \"../utils.typ\": f; #f()!");
    project.write("utils.typ", "#let f() = panic(42)");
    let output = exec().arg("compile").arg(&main).must_fail();
    output.stderr.must_contain("error: panicked with: 42");
}

#[test]
fn test_path_unresolved() {
    let project = tempfs();
    let main = project.write("main.typ", "#include \"other.typ\"");
    let output = exec().arg("compile").arg(&main).must_fail();
    output
        .stderr
        .must_contain("error: file not found")
        .must_contain("#include \"other.typ\"");
}

#[test]
fn test_path_project_root() {
    let project = tempfs();
    let main = project.write("src/main.typ", "#include \"/a.typ\"");
    project.write("a.typ", "#panic(42)");
    let output = exec()
        .arg("compile")
        .arg(&main)
        .arg("--root")
        .arg(project.path())
        .must_fail();
    output.stderr.must_contain("error: panicked with: 42");
}

#[test]
fn test_package_resolved() {
    let project = tempfs();
    let package = tempfs();
    let main = project.write("main.typ", "#import \"@local/demo:0.1.0\": f; #f()");
    package.write(
        "local/demo/0.1.0/typst.toml",
        r#"[package]
           name = "demo"
           version = "0.1.0"
           entrypoint = "lib.typ""#,
    );
    package.write("local/demo/0.1.0/lib.typ", "#import \"utils.typ\": f");
    package.write("local/demo/0.1.0/utils.typ", "#let f() = panic(42)");
    let output = exec()
        .arg("compile")
        .arg(&main)
        .arg("--package-path")
        .arg(package.path())
        .must_fail();
    output.stderr.must_contain("error: panicked with: 42");
}

#[test]
fn test_package_unresolved() {
    let project = tempfs();
    let package = tempfs();
    let main = project.write("main.typ", "#import \"@local/demo:0.1.0\": f; #f()");
    let output = exec()
        .arg("compile")
        .arg(&main)
        .arg("--package-path")
        .arg(package.path())
        .must_fail();
    output
        .stderr
        .must_contain("error: package not found (searched for @local/demo:0.1.0)");
}

#[test]
fn test_path_to_package() {
    let project = tempfs();
    let package = tempfs();
    let main = project.write(
        "main.typ",
        "#import \"@local/demo:0.1.0\": g
         #let x = g(path(\"a.typ\")) // from project
         #let y = g(\"a.typ\")       // from package
         #panic((x, y))",
    );
    project.write("a.typ", "#let f() = 7");
    package.write(
        "local/demo/0.1.0/typst.toml",
        r#"[package]
           name = "demo"
           version = "0.1.0"
           entrypoint = "lib.typ""#,
    );
    package.write("local/demo/0.1.0/lib.typ", "#let g(p) = { import p: f; f() }");
    package.write("local/demo/0.1.0/a.typ", "#let f() = 42");
    let output = exec()
        .arg("compile")
        .arg(&main)
        .arg("--package-path")
        .arg(package.path())
        .must_fail();
    output.stderr.must_contain("error: panicked with: (7, 42)");
}

#[test]
fn test_network_access_hint() {
    // Using a CLI test because the error message differs across operating
    // systems. If the test runner could handle that, we could migrate to a
    // normal test.
    let project = tempfs();
    let main = project.write("main.typ", "#image(\"https://example.org/image.png\")");
    let output = exec().arg("compile").arg(main).must_fail();
    output.stderr.must_contain("hint: network access is not supported");
}

/// Executes a command with the Typst CLI.
fn exec() -> Command {
    Command::new(env!("CARGO_BIN_EXE_typst"))
}

trait CommandExt {
    fn must_succeed(&mut self) -> TestOutput;
    fn must_fail(&mut self) -> TestOutput;
}

impl CommandExt for Command {
    #[track_caller]
    fn must_succeed(&mut self) -> TestOutput {
        let output = self.output().unwrap();
        assert!(
            output.status.success(),
            "process failed ({}):\n{}",
            output.status,
            Stream(output.stderr),
        );
        output.into()
    }

    #[track_caller]
    fn must_fail(&mut self) -> TestOutput {
        let output = self.output().unwrap();
        assert!(!output.status.success(), "process succeeded ({})", output.status);
        output.into()
    }
}

struct TestOutput {
    stdout: Stream,
    stderr: Stream,
}

impl From<Output> for TestOutput {
    fn from(value: Output) -> Self {
        Self {
            stdout: Stream(value.stdout),
            stderr: Stream(value.stderr),
        }
    }
}

#[track_caller]
fn tempfs() -> TempFs {
    TempFs(tempfile::tempdir().unwrap())
}

struct TempFs(TempDir);

impl TempFs {
    fn path(&self) -> &Path {
        self.0.path()
    }

    fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        self.path().join(path)
    }

    #[track_caller]
    fn write(&self, path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> PathBuf {
        let full = self.resolve(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full, data).unwrap();
        full
    }

    #[track_caller]
    fn read(&self, path: impl AsRef<Path>) -> Stream<Vec<u8>> {
        Stream(std::fs::read(self.resolve(path)).unwrap())
    }
}

struct Stream<T = Vec<u8>>(T);

impl<T: AsRef<[u8]>> Stream<T> {
    #[track_caller]
    fn must_contain(&self, data: impl Debug + AsRef<[u8]>) -> &Self {
        assert!(self.contains(data.as_ref()), "{self:?} did not contain {data:?}",);
        self
    }

    #[track_caller]
    fn must_start_with(&self, data: impl Debug + AsRef<[u8]>) -> &Self {
        assert!(
            self.0.as_ref().starts_with(data.as_ref()),
            "{self:?} did not start with {data:?}",
        );
        self
    }

    #[track_caller]
    fn must_match_lines<'s>(&self, lines: impl IntoIterator<Item = &'s str>) -> &Self {
        assert_eq!(
            self.lines().collect::<Vec<_>>(),
            lines.into_iter().collect::<Vec<_>>(),
        );
        self
    }

    fn contains(&self, data: impl AsRef<[u8]>) -> bool {
        memchr::memmem::find(self.0.as_ref(), data.as_ref()).is_some()
    }

    fn lines(&self) -> impl Iterator<Item = &str> {
        std::str::from_utf8(self.0.as_ref())
            .unwrap_or_else(|_| panic!("{self} is not valid utf-8"))
            .lines()
    }
}

impl<T: AsRef<[u8]>> Debug for Stream<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&String::from_utf8_lossy(self.0.as_ref()), f)
    }
}

impl<T: AsRef<[u8]>> Display for Stream<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&String::from_utf8_lossy(self.0.as_ref()), f)
    }
}
