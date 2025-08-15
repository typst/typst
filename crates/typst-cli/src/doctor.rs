use std::env::VarError;
use std::fmt::Display;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::terminal::{self, TermOut};
use clap::ValueEnum;
use codespan_reporting::term::termcolor::{Color, ColorSpec, WriteColor};
use ecow::eco_format;
use serde::Serialize;
use typst::diag::StrResult;

use crate::args::{DoctorCommand, Feature};

/// A struct holding the machine readable output of the environment command.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Doctor {
    /// General info about Typst.
    general_info: GeneralInfo,

    /// The runtime features from `TYPST_FEATURES`.
    features: Features,

    /// Paths typst will use.
    paths: Paths,

    /// The environment variables that are of interest to Typst.
    env: Environment,
}

/// General info about Typst.
#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct GeneralInfo {
    /// The Typst version.
    version: &'static str,

    /// Whether `self-update` is enabled.
    self_update: bool,

    /// Whether `http-server` is enabled.
    http_server: bool,
}

impl GeneralInfo {
    fn compile_features(&self) -> impl Iterator<Item = (&'static str, bool)> {
        let Self { version: _, self_update, http_server } = self;

        [("self-update", self_update), ("http-server", http_server)]
            .into_iter()
            .map(|(k, v)| (k, *v))
    }
}

/// The runtime features from `TYPST_FEATURES`.
#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Features {
    html: bool,
}

impl Features {
    fn features(&self) -> impl Iterator<Item = (&'static str, bool)> {
        let Self { html } = self;

        [("html", html)].into_iter().map(|(k, v)| (k, *v))
    }
}

/// Paths typst will use.
#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Paths {
    /// The font paths from `TYPST_FONT_PATHS`.
    font_paths: Vec<PathBuf>,

    /// The resolved package path.
    package_path: Option<PathBuf>,

    /// The resolved package cache path.
    package_cache_path: Option<PathBuf>,
}

impl Paths {
    fn package_paths(&self) -> impl Iterator<Item = (&'static str, Option<&'_ Path>)> {
        let Self { font_paths: _, package_path, package_cache_path } = self;

        [("package-path", package_path), ("package-cache-path", package_cache_path)]
            .into_iter()
            .map(|(k, v)| (k, v.as_deref()))
    }
}

/// The environment variables that are of interest to Typst.
#[derive(Default, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct Environment {
    source_date_epoch: Option<String>,
    typst_cert: Option<String>,
    typst_features: Option<String>,
    typst_font_paths: Option<String>,
    typst_ignore_system_fonts: Option<String>,
    typst_package_cache_path: Option<String>,
    typst_package_path: Option<String>,
    typst_root: Option<String>,
    typst_update_backup_path: Option<String>,
    #[cfg(target_os = "linux")]
    xdg_cache_home: Option<String>,
    #[cfg(target_os = "linux")]
    xdg_data_home: Option<String>,
}

impl Environment {
    fn vars(&self) -> impl Iterator<Item = (&'static str, Option<&'_ str>)> {
        let Environment {
            source_date_epoch,
            typst_cert,
            typst_features,
            typst_font_paths,
            typst_ignore_system_fonts,
            typst_package_cache_path,
            typst_package_path,
            typst_root,
            typst_update_backup_path,
            #[cfg(target_os = "linux")]
            xdg_cache_home,
            #[cfg(target_os = "linux")]
            xdg_data_home,
        } = self;

        [
            ("SOURCE_DATE_EPOCH", source_date_epoch),
            ("TYPST_CERT", typst_cert),
            ("TYPST_FEATURES", typst_features),
            ("TYPST_FONT_PATHS", typst_font_paths),
            ("TYPST_IGNORE_SYSTEM_FONTS", typst_ignore_system_fonts),
            ("TYPST_PACKAGE_CACHE_PATH", typst_package_cache_path),
            ("TYPST_PACKAGE_PATH", typst_package_path),
            ("TYPST_ROOT", typst_root),
            ("TYPST_UPDATE_BACKUP_PATH", typst_update_backup_path),
            #[cfg(target_os = "linux")]
            ("XDG_CACHE_HOME", xdg_cache_home),
            #[cfg(target_os = "linux")]
            ("XDG_DATA_HOME", xdg_data_home),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.as_deref()))
    }
}

pub fn doctor(command: &DoctorCommand) -> StrResult<()> {
    let env = get_vars()?;

    let runtime_features =
        parse_features(env.typst_features.as_deref().unwrap_or_default())?;

    let font_paths = env
        .typst_font_paths
        .as_deref()
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect::<_>();

    let value = Doctor {
        general_info: GeneralInfo {
            version: crate::typst_version(),
            self_update: cfg!(feature = "self-update"),
            http_server: cfg!(feature = "http-server"),
        },
        features: runtime_features,
        paths: Paths {
            font_paths,
            package_path: env
                .typst_package_path
                .as_ref()
                .map(PathBuf::from)
                .or_else(typst_kit::package::default_package_path),
            package_cache_path: env
                .typst_package_cache_path
                .as_ref()
                .map(PathBuf::from)
                .or_else(typst_kit::package::default_package_cache_path),
        },
        env,
    };

    if let Some(format) = command.format {
        let serialized = crate::serialize(&value, format, command.pretty)?;
        println!("{serialized}");
    } else {
        format_human_readable(&value).map_err(|e| eco_format!("{e}"))?;
    }

    Ok(())
}

fn format_human_readable(value: &Doctor) -> io::Result<()> {
    fn write_key(
        out: &mut TermOut,
        key: impl Display,
        pad_to_len: Option<usize>,
    ) -> io::Result<()> {
        out.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
        if let Some(pad) = pad_to_len {
            write!(out, "{key: <pad$}")?;
        } else {
            write!(out, "{key}")?;
        }
        out.reset()?;

        Ok(())
    }

    fn write_value_simple(out: &mut TermOut, val: impl Display) -> io::Result<()> {
        out.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(out, "{val}")?;
        out.reset()?;

        Ok(())
    }

    fn write_value_special(out: &mut TermOut, val: impl Display) -> io::Result<()> {
        out.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
        write!(out, "{val}")?;
        out.reset()?;

        Ok(())
    }

    fn get_padding<'a, T>(pairs: impl Iterator<Item = (&'a str, T)>) -> usize {
        pairs.map(|(k, _)| k.len()).max().unwrap_or_default()
    }

    let mut out = terminal::out();

    writeln!(out, "General Information")?;
    write!(out, "  ")?;
    write_key(&mut out, "Version", None)?;
    write!(out, " ")?;
    write_value_simple(&mut out, value.general_info.version)?;
    writeln!(out)?;
    let pad = get_padding(value.general_info.compile_features());
    for (key, val) in value.general_info.compile_features() {
        write!(out, "  ")?;
        write_key(&mut out, key, Some(pad))?;
        write!(out, " ")?;
        write_value_special(&mut out, if val { "on" } else { "off" })?;
        writeln!(out)?;
    }

    writeln!(out)?;
    writeln!(out, "Features")?;
    let pad = get_padding(value.features.features());
    for (key, val) in value.features.features() {
        write!(out, "  ")?;
        write_key(&mut out, key, Some(pad))?;
        write!(out, " ")?;
        write_value_special(&mut out, if val { "on" } else { "off" })?;
        writeln!(out)?;
    }

    writeln!(out)?;
    writeln!(out, "Paths")?;
    let pad = get_padding(value.paths.package_paths());
    for (key, val) in value.paths.package_paths() {
        write!(out, "  ")?;
        write_key(&mut out, key, Some(pad))?;
        write!(out, " ")?;

        if let Some(val) = val {
            write_value_simple(&mut out, val.display())?;
        } else {
            write_value_special(&mut out, "<unset>")?;
        }

        writeln!(out)?;
    }
    if value.paths.font_paths.is_empty() {
        write!(out, "  ")?;
        write_key(&mut out, "font-paths", Some(pad))?;
        write!(out, " ")?;
        write_value_special(&mut out, "<none>")?;
        writeln!(out)?;
    } else {
        writeln!(out, "  font-paths")?;
        for path in &value.paths.font_paths {
            write!(out, "    ")?;
            write_value_simple(&mut out, path.display())?;
            writeln!(out)?;
        }
    }

    writeln!(out)?;
    writeln!(out, "Environment Variables")?;
    let pad = get_padding(value.env.vars());
    for (key, val) in value.env.vars() {
        write!(out, "  ")?;
        write_key(&mut out, key, Some(pad))?;
        write!(out, " ")?;

        if let Some(val) = val {
            write_value_simple(&mut out, val)?;
        } else {
            write_value_special(&mut out, "<unset>")?;
        }

        writeln!(out)?;
    }

    Ok(())
}

/// Retrieves all relevant environment variables.
fn get_vars() -> StrResult<Environment> {
    fn get_var(key: &'static str) -> StrResult<Option<String>> {
        match std::env::var(key) {
            Ok(val) => Ok(Some(val)),
            Err(VarError::NotPresent) => Ok(None),
            Err(VarError::NotUnicode(_)) => {
                crate::set_failed();
                crate::print_error(&format!(
                    "the environment variable `{key}` was not valid UTF-8"
                ))
                .map_err(|e| eco_format!("{e}"))?;
                Ok(None)
            }
        }
    }

    Ok(Environment {
        source_date_epoch: get_var("SOURCE_DATE_EPOCH")?,
        typst_cert: get_var("TYPST_CERT")?,
        typst_features: get_var("TYPST_FEATURES")?,
        typst_font_paths: get_var("TYPST_FONT_PATHS")?,
        typst_ignore_system_fonts: get_var("TYPST_IGNORE_SYSTEM_FONTS")?,
        typst_package_cache_path: get_var("TYPST_PACKAGE_CACHE_PATH")?,
        typst_package_path: get_var("TYPST_PACKAGE_PATH")?,
        typst_root: get_var("TYPST_ROOT")?,
        typst_update_backup_path: get_var("TYPST_UPDATE_BACKUP_PATH")?,
        #[cfg(target_os = "linux")]
        xdg_cache_home: get_var("XDG_CACHE_HOME")?,
        #[cfg(target_os = "linux")]
        xdg_data_home: get_var("XDG_DATA_HOME")?,
    })
}

/// Turns a comma separated list of feature names into a well typed struct of
/// feature flags.
fn parse_features(feature_list: &str) -> StrResult<Features> {
    let mut features = Features { html: false };

    for feature in feature_list.split(',').filter(|s| !s.is_empty()) {
        match Feature::from_str(feature, true) {
            Ok(feature) => match feature {
                Feature::Html => features.html = true,
            },
            Err(_) => {
                crate::print_error(&format!("Unknown runtime feature: `{feature}`"))
                    .map_err(|e| eco_format!("{e}"))?;
                continue;
            }
        }
    }

    Ok(features)
}
