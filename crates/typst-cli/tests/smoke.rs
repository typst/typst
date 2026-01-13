use std::io;
use std::process::Command;

use tempfile::tempdir;

/// Executes a command with the Typst CLI.
fn exec() -> Command {
    Command::new(env!("CARGO_BIN_EXE_typst"))
}

#[test]
fn test_help() -> io::Result<()> {
    let output = exec().arg("--help").output()?;
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    assert!(stdout.contains("Compiles an input file"));
    assert!(stdout.contains("https://typst.app/docs/tutorial/"));
    Ok(())
}

#[test]
fn test_compile_pdf() -> io::Result<()> {
    let tmp = tempdir()?;
    let title = "Hello from CLI";

    let typ_path = tmp.path().join("hello.typ");
    std::fs::write(&typ_path, format!(r#"#set document(title: "{title}")"#))?;
    let status = exec().arg("compile").arg(&typ_path).status()?;
    assert!(status.success());

    // Basic sanity checks on the PDF:
    // It should start PDF-like and contain our title in its XMP metadata.
    let pdf_path = tmp.path().join("hello.pdf");
    let pdf = std::fs::read(&pdf_path)?;
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.windows(title.len()).any(|s| s == title.as_bytes()));

    Ok(())
}

#[test]
fn test_eval() -> io::Result<()> {
    let output = exec().arg("eval").arg("1+2").output()?;
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(stdout.trim(), "3");
    Ok(())
}
