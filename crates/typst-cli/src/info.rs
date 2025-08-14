use std::env::VarError;
use std::fmt::Display;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::builder::{FalseyValueParser, TypedValueParser};
use clap::{CommandFactory, ValueEnum};
use codespan_reporting::term::termcolor::{Color, ColorSpec, WriteColor};
use ecow::eco_format;
use serde::Serialize;
use typst::diag::StrResult;

use crate::CliArguments;
use crate::args::{Feature, InfoCommand};
use crate::terminal::{self, TermOut};

/// A struct holding the machine readable output of the environment command.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Info {
    /// The Typst version.
    version: &'static str,

    /// Build info about Typst.
    build: Build,

    /// The runtime features from `TYPST_FEATURES`.
    features: Features,

    /// Font configuration.
    fonts: Fonts,

    /// Package configuration.
    packages: Packages,

    /// The environment variables that are of interest to Typst.
    env: Environment,
}

/// Build info about Typst.
#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Build {
    /// The commit this binary was compiled with.
    commit: &'static str,

    /// Compile time settings.
    settings: Settings,
}

/// Compile time settings.
#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Settings {
    /// Whether the `self-update` compile-time feature is enabled.
    self_update: bool,

    /// Whether the `http-server` compile-time feature is enabled.
    http_server: bool,
}

impl Settings {
    /// Return the compile features with human readable information.
    fn compile_features(&self) -> impl Iterator<Item = KeyValDesc<'_>> {
        let Self { self_update, http_server } = self;

        [
            ("self-update", self_update, "Update typst via `typst update`"),
            ("http-server", http_server, "Serve HTML via `typst watch`"),
        ]
        .into_iter()
        .map(|(key, val, desc)| KeyValDesc { key, val: Value::Bool(*val), desc })
    }
}

/// The runtime features from `TYPST_FEATURES`.
#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Features {
    html: bool,
    a11y_extras: bool,
}

impl Features {
    /// Return the runtime features with human readable information.
    fn features(&self) -> impl Iterator<Item = KeyValDesc<'_>> {
        let Self { html, a11y_extras } = self;

        [
            ("html", html, "Experimental HTML support"),
            ("a11y-extras", a11y_extras, "Experimental PDF accessibility extensions"),
        ]
        .into_iter()
        .map(|(key, val, desc)| KeyValDesc { key, val: Value::Bool(*val), desc })
    }
}

/// Font configuration.
#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Fonts {
    /// The font paths from `TYPST_FONT_PATHS`.
    paths: Vec<PathBuf>,

    /// Whether system fonts were included in the search.
    system: bool,

    /// Whether embedded fonts were included in the search.
    embedded: bool,
}

impl Fonts {
    /// Return the custom font paths.
    fn custom_paths(&self) -> impl Iterator<Item = Value<'_>> {
        self.paths.iter().map(|p| Value::Path(p))
    }

    /// Return whether system and embedded fonts are included.
    fn included(&self) -> impl Iterator<Item = (&'static str, Value<'_>)> {
        let Self { paths: _, system, embedded } = self;

        [("System fonts", system), ("Embedded fonts", embedded)]
            .into_iter()
            .map(|(key, val)| (key, Value::Bool(*val)))
    }
}

/// Package configuration.
#[derive(Default, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Packages {
    /// The resolved package path.
    package_path: Option<PathBuf>,

    /// The resolved package cache path.
    package_cache_path: Option<PathBuf>,
}

impl Packages {
    /// Return the resolved package paths.
    fn paths(&self) -> impl Iterator<Item = (&'static str, Value<'_>)> {
        let Self { package_path, package_cache_path } = self;

        [("Package path", package_path), ("Package cache path", package_cache_path)]
            .into_iter()
            .map(|(k, v)| (k, v.as_deref().map(Value::Path).unwrap_or(Value::Unset)))
    }
}

/// The environment variables that are of interest to Typst.
#[derive(Default, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct Environment {
    typst_cert: Option<String>,
    typst_features: Option<String>,
    typst_font_paths: Option<String>,
    typst_ignore_system_fonts: Option<String>,
    typst_package_cache_path: Option<String>,
    typst_package_path: Option<String>,
    typst_root: Option<String>,
    typst_update_backup_path: Option<String>,
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
    xdg_cache_home: Option<String>,
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
    xdg_data_home: Option<String>,
    source_date_epoch: Option<String>,
    fontconfig_file: Option<String>,
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
    openssl_conf: Option<String>,
    no_color: Option<String>,
    no_proxy: Option<String>,
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    all_proxy: Option<String>,
}

impl Environment {
    fn vars(&self) -> impl Iterator<Item = (&'static str, Value<'_>)> {
        let Environment {
            typst_cert,
            typst_features,
            typst_font_paths,
            typst_ignore_system_fonts,
            typst_package_cache_path,
            typst_package_path,
            typst_root,
            typst_update_backup_path,
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
            )))]
            xdg_cache_home,
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
            )))]
            xdg_data_home,
            source_date_epoch,
            fontconfig_file,
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
            )))]
            openssl_conf,
            no_color,
            no_proxy,
            http_proxy,
            https_proxy,
            all_proxy,
        } = self;

        [
            ("TYPST_CERT", typst_cert),
            ("TYPST_FEATURES", typst_features),
            ("TYPST_FONT_PATHS", typst_font_paths),
            ("TYPST_IGNORE_SYSTEM_FONTS", typst_ignore_system_fonts),
            ("TYPST_PACKAGE_CACHE_PATH", typst_package_cache_path),
            ("TYPST_PACKAGE_PATH", typst_package_path),
            ("TYPST_ROOT", typst_root),
            ("TYPST_UPDATE_BACKUP_PATH", typst_update_backup_path),
            ("SOURCE_DATE_EPOCH", source_date_epoch),
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
            )))]
            ("XDG_CACHE_HOME", xdg_cache_home),
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
            )))]
            ("XDG_DATA_HOME", xdg_data_home),
            ("FONTCONFIG_FILE", fontconfig_file),
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
            )))]
            ("OPENSSL_CONF", openssl_conf),
            ("NO_COLOR", no_color),
            ("NO_PROXY", no_proxy),
            ("HTTP_PROXY", http_proxy),
            ("HTTPS_PROXY", https_proxy),
            ("ALL_PROXY", all_proxy),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.as_deref().map(Value::String).unwrap_or(Value::Unset)))
    }
}

pub fn info(command: &InfoCommand) -> StrResult<()> {
    let cmd = CliArguments::command();

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

    let value = Info {
        version: crate::typst_version(),
        build: Build {
            commit: crate::typst_commit_sha(),
            settings: Settings {
                self_update: cfg!(feature = "self-update"),
                http_server: cfg!(feature = "http-server"),
            },
        },
        features: runtime_features,
        fonts: Fonts {
            paths: font_paths,
            system: !env
                .typst_ignore_system_fonts
                .as_ref()
                .and_then(|v| {
                    // This is only an error if `v` is not valid UTF-8, which it
                    // always is.
                    FalseyValueParser::new().parse_ref(&cmd, None, v.as_ref()).ok()
                })
                .unwrap_or_default(),
            embedded: true,
        },
        packages: Packages {
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
        typst_cert: get_var("TYPST_CERT")?,
        typst_features: get_var("TYPST_FEATURES")?,
        typst_font_paths: get_var("TYPST_FONT_PATHS")?,
        typst_ignore_system_fonts: get_var("TYPST_IGNORE_SYSTEM_FONTS")?,
        typst_package_cache_path: get_var("TYPST_PACKAGE_CACHE_PATH")?,
        typst_package_path: get_var("TYPST_PACKAGE_PATH")?,
        typst_root: get_var("TYPST_ROOT")?,
        typst_update_backup_path: get_var("TYPST_UPDATE_BACKUP_PATH")?,
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
        xdg_cache_home: get_var("XDG_CACHE_HOME")?,
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
        xdg_data_home: get_var("XDG_DATA_HOME")?,
        source_date_epoch: get_var("SOURCE_DATE_EPOCH")?,
        fontconfig_file: get_var("FONTCONFIG_FILE")?,
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios")))]
        openssl_conf: get_var("OPENSSL_CONF")?,
        no_color: get_var("NO_COLOR")?,
        no_proxy: get_var("NO_PROXY")?,
        http_proxy: get_var("HTTP_PROXY")?,
        https_proxy: get_var("HTTPS_PROXY")?,
        all_proxy: get_var("ALL_PROXY")?,
    })
}

/// Turns a comma separated list of feature names into a well typed struct of
/// feature flags.
fn parse_features(feature_list: &str) -> StrResult<Features> {
    let mut features = Features { html: false, a11y_extras: false };

    for feature in feature_list.split(',').filter(|s| !s.is_empty()) {
        match Feature::from_str(feature, true) {
            Ok(feature) => match feature {
                Feature::Html => features.html = true,
                Feature::A11yExtras => features.a11y_extras = true,
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

/// A for formatting human readable key-value-description triplets.
struct KeyValDesc<'a> {
    key: &'static str,
    val: Value<'a>,
    desc: &'static str,
}

impl KeyValDesc<'_> {
    /// Formatted this as `<key> <val> (<desc>)` with optional right padding for
    /// key and value.
    fn format(
        &self,
        out: &mut TermOut,
        key_pad: Option<usize>,
        val_pad: Option<usize>,
    ) -> io::Result<()> {
        write!(out, "  ")?;
        write_key(out, self.key, key_pad)?;
        write!(out, " ")?;
        self.val.format(out, val_pad)?;
        write!(out, " ({})", self.desc)?;

        Ok(())
    }
}

/// A value for colorful human readable formatting.
enum Value<'a> {
    Unset,
    Bool(bool),
    Path(&'a Path),
    String(&'a str),
}

impl Value<'_> {
    /// Formats this value with optional right padding.
    fn format(&self, out: &mut TermOut, pad: Option<usize>) -> io::Result<()> {
        match self {
            Value::Unset => write_value_special(out, "<unset>", pad),
            Value::Bool(true) => write_value_special(out, "on", pad),
            Value::Bool(false) => write_value_special(out, "off", pad),
            Value::Path(val) => write_value_simple(out, val.display(), pad),
            Value::String(val) => write_value_simple(out, val, pad),
        }
    }
}

/// Writes a key in cyan with optional right padding.
fn write_key(out: &mut TermOut, key: impl Display, pad: Option<usize>) -> io::Result<()> {
    out.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
    if let Some(pad) = pad {
        write!(out, "{key: <pad$}")?;
    } else {
        write!(out, "{key}")?;
    }
    out.reset()?;

    Ok(())
}

/// Writes a value in green with optional right padding.
fn write_value_simple(
    out: &mut TermOut,
    val: impl Display,
    pad: Option<usize>,
) -> io::Result<()> {
    out.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    if let Some(pad) = pad {
        write!(out, "{val: <pad$}")?;
    } else {
        write!(out, "{val}")?;
    }
    out.reset()?;

    Ok(())
}

/// Writes a special value in magenta with optional right padding.
fn write_value_special(
    out: &mut TermOut,
    val: impl Display,
    pad: Option<usize>,
) -> io::Result<()> {
    out.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
    if let Some(pad) = pad {
        write!(out, "{val: <pad$}")?;
    } else {
        write!(out, "{val}")?;
    }
    out.reset()?;

    Ok(())
}

fn format_human_readable(value: &Info) -> io::Result<()> {
    let mut out = terminal::out();

    write_key(&mut out, "Version", None)?;
    write!(out, " ")?;
    write_value_simple(&mut out, value.version, None)?;
    write!(out, " (")?;
    write_value_simple(&mut out, value.build.commit, None)?;
    writeln!(out, ")\n")?;

    writeln!(out, "Build settings")?;
    let key_pad = value.build.settings.compile_features().map(|f| f.key.len()).max();
    for feature in value.build.settings.compile_features() {
        feature.format(&mut out, key_pad, Some(3))?;
        writeln!(out)?;
    }

    writeln!(out)?;
    writeln!(out, "Features")?;
    let key_pad = value.features.features().map(|f| f.key.len()).max();
    for feature in value.features.features() {
        feature.format(&mut out, key_pad, Some(3))?;
        writeln!(out)?;
    }

    writeln!(out)?;
    writeln!(out, "Fonts")?;
    write!(out, "  ")?;
    write_key(&mut out, "Custom font paths", None)?;
    if value.fonts.paths.is_empty() {
        write!(out, " ")?;
        write_value_special(&mut out, "<none>", None)?;
        writeln!(out)?;
    } else {
        writeln!(out)?;
        for path in value.fonts.custom_paths() {
            write!(out, "    - ")?;
            path.format(&mut out, None)?;
            writeln!(out)?;
        }
    }

    let key_pad = value.fonts.included().map(|(key, _)| key.len()).max();
    for (key, val) in value.fonts.included() {
        write!(out, "  ")?;
        write_key(&mut out, key, key_pad)?;
        write!(out, " ")?;
        val.format(&mut out, None)?;
        writeln!(out)?;
    }

    writeln!(out)?;
    writeln!(out, "Packages")?;
    let key_pad = value.packages.paths().map(|(name, _)| name.len()).max();
    for (key, val) in value.packages.paths() {
        write!(out, "  ")?;
        write_key(&mut out, key, key_pad)?;
        write!(out, " ")?;
        val.format(&mut out, None)?;
        writeln!(out)?;
    }

    writeln!(out)?;
    writeln!(out, "Environment variables")?;
    let key_pad = value.env.vars().map(|(name, _)| name.len()).max();
    for (key, val) in value.env.vars() {
        write!(out, "  ")?;
        write_key(&mut out, key, key_pad)?;
        write!(out, " ")?;
        val.format(&mut out, None)?;

        writeln!(out)?;
    }

    Ok(())
}
