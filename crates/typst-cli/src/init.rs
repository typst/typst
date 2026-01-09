use std::io::Write;
use std::path::Path;

use codespan_reporting::term::termcolor::{Color, ColorSpec, WriteColor};
use ecow::eco_format;
use fs_extra::dir::CopyOptions;
use typst::diag::{FileError, StrResult, bail};
use typst::syntax::package::{
    PackageManifest, PackageSpec, TemplateInfo, VersionlessPackageSpec,
};

use crate::args::InitCommand;
use crate::download::PrintDownload;
use crate::package;

/// Execute an initialization command.
pub fn init(command: &InitCommand) -> StrResult<()> {
    let package_storage = package::storage(&command.package);

    // Parse the package specification. If the user didn't specify the version,
    // we try to figure it out automatically by downloading the package index
    // or searching the disk.
    let spec: PackageSpec = command.template.parse().or_else(|err| {
        // Try to parse without version, but prefer the error message of the
        // normal package spec parsing if it fails.
        let spec: VersionlessPackageSpec = command.template.parse().map_err(|_| err)?;
        let version = package_storage.determine_latest_version(&spec)?;
        StrResult::Ok(spec.at(version))
    })?;

    // Find or download the package.
    let package_path =
        package_storage.prepare_package(&spec, &mut PrintDownload(&spec))?;

    // Parse the manifest.
    let manifest = parse_manifest(&package_path)?;
    manifest.validate(&spec)?;

    // Ensure that it is indeed a template.
    let Some(template) = &manifest.template else {
        bail!("package {spec} is not a template");
    };

    // Determine the directory at which we will create the project.
    let project_dir = Path::new(command.dir.as_deref().unwrap_or(&manifest.package.name));

    // Set up the project.
    scaffold_project(project_dir, &package_path, template)?;

    // Print the summary.
    print_summary(spec, project_dir, template).unwrap();

    Ok(())
}

/// Parses the manifest of the package located at `package_path`.
fn parse_manifest(package_path: &Path) -> StrResult<PackageManifest> {
    let toml_path = package_path.join("typst.toml");
    let string = std::fs::read_to_string(&toml_path).map_err(|err| {
        eco_format!(
            "failed to read package manifest ({})",
            FileError::from_io(err, &toml_path)
        )
    })?;

    toml::from_str(&string)
        .map_err(|err| eco_format!("package manifest is malformed ({})", err.message()))
}

/// Creates the project directory with the template's contents and returns the
/// path at which it was created.
fn scaffold_project(
    project_dir: &Path,
    package_path: &Path,
    template: &TemplateInfo,
) -> StrResult<()> {
    let template_dir = package_path.join(template.path.as_str());
    if !template_dir.exists() {
        bail!("template directory does not exist (at {})", template_dir.display());
    }

    fs_extra::dir::copy(
        &template_dir,
        project_dir,
        &CopyOptions::new().content_only(true),
    )
    .map_err(|err| eco_format!("failed to create project directory ({err})"))?;

    Ok(())
}

/// Prints a summary after successful initialization.
fn print_summary(
    spec: PackageSpec,
    project_dir: &Path,
    template: &TemplateInfo,
) -> std::io::Result<()> {
    let mut gray = ColorSpec::new();
    gray.set_fg(Some(Color::White));
    gray.set_dimmed(true);

    let mut out = crate::terminal::out();
    writeln!(out, "Successfully created new project from {spec} 🎉")?;
    writeln!(out, "To start writing, run:")?;
    out.set_color(&gray)?;
    write!(out, "> ")?;
    out.reset()?;
    writeln!(
        out,
        "cd {}",
        shell_escape::escape(project_dir.display().to_string().into()),
    )?;
    out.set_color(&gray)?;
    write!(out, "> ")?;
    out.reset()?;
    writeln!(
        out,
        "typst watch {}",
        shell_escape::escape(template.entrypoint.to_string().into()),
    )?;
    writeln!(out)?;
    Ok(())
}
