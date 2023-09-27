mod logic;
mod output;
pub mod settings;
mod state;
pub mod styles;

use ecow::EcoString;
use settings::Settings;
use state::State;
use typst_syntax::{SyntaxKind, SyntaxNode};
type StrResult<T> = Result<T, EcoString>;

use output::Output;

pub use output::OutputTarget;
use std::{
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

pub use styles::Styles;

use clap::Parser;

const CONFIG_NAME: &str = "typstfmt.toml";

#[derive(Debug, Clone, Parser)]
pub struct Command {
    /// Input path for source file, used as output path if nothing else is specified
    #[arg(default_value = None)]
    pub path: Option<PathBuf>,

    /// Output path
    #[arg(short, long, default_value = None)]
    pub output: Option<PathBuf>,

    /// Base style for the formatting settings
    #[arg(short, long, default_value_t = Styles::Default)]
    pub style: Styles,

    /// Search for 'typstfmt.toml' for additional formatting settings
    #[arg(long, default_value_t = false)]
    pub use_configuration: bool,

    /// Generate file with formatting settings based on the style
    #[arg(long, default_value_t = false)]
    pub save_configuration: bool,

    /// Use standard input as source
    #[arg(long, default_value_t = false)]
    pub use_std_in: bool,

    /// Use standard output as target
    #[arg(long, default_value_t = false)]
    pub use_std_out: bool,

    /// File location to search for configuration, defaults to input path if available
    #[arg(long, default_value = None)]
    pub file_location: Option<PathBuf>,
}

pub fn format_node(
    node: &SyntaxNode,
    settings: &settings::Settings,
    target: &mut impl OutputTarget,
) -> StrResult<()> {
    let mut output = Output::new(target);
    let state = State::new();
    logic::format(node, state, settings, &mut output);

    // ensure end of file is always present
    logic::format(
        &SyntaxNode::leaf(SyntaxKind::Eof, EcoString::new()),
        state,
        settings,
        &mut output,
    );
    output.finish(&state, settings);
    Ok(())
}

pub fn format_str(
    text: &str,
    settings: &settings::Settings,
    target: &mut impl OutputTarget,
) -> StrResult<()> {
    format_node(&typst_syntax::parse(text), settings, target)
}

pub fn format(command: &Command) -> StrResult<()> {
    let mut settings = command.style.settings();

    if command.use_configuration {
        let path = match (&command.file_location, &command.path) {
            (Some(path), _) => {
                if path.extension().is_some() {
                    path.parent().ok_or("failed to get parent folder")?.to_owned()
                } else {
                    path.to_owned()
                }
            }
            (_, Some(path)) => path.to_owned(),
            _ => std::env::current_dir().unwrap().to_owned(),
        };
        let mut path = path.as_path();
        let file = loop {
            let mut file = PathBuf::from(path);
            file.push(CONFIG_NAME);
            if file.is_file() {
                break file;
            }
            path = path.parent().ok_or("could not find 'typstfmt.toml'")?;
        };
        settings.merge(&file)?;
    }

    if command.save_configuration {
        std::fs::write(
            CONFIG_NAME,
            toml::to_string_pretty(&settings).map_err(|err| err.to_string())?,
        )
        .map_err(|_| "could not save configuration")?;
        return Ok(());
    }

    let (input_data, input_name) = match (&command.path, command.use_std_in) {
        (Some(_), true) => return Err("input path and stdin are incompatible".into()),
        (Some(path), false) => {
            let input_data = std::fs::read_to_string(path)
                .map_err(|_| format!("could not read '{}'", path.display()))?;
            (input_data, path.display().to_string())
        }
        (None, true) => {
            let mut data = String::new();
            std::io::stdin()
                .read_to_string(&mut data)
                .map_err(|err| err.to_string())?;
            (data, "stdin".into())
        }
        (None, false) => return Err("no input path or stdin specified".into()),
    };

    let root = typst_syntax::parse(&input_data);

    match (&command.output, command.use_std_out) {
        (Some(_), true) => return Err("output path and stdout are incompatible".into()),
        (Some(out), false) => {
            let file = File::create(&out)
                .map_err(|_| format!("could not create '{}'", out.display()))?;
            let mut target = FileTarget(BufWriter::new(file));
            format_node(&root, &settings, &mut target)?;
            drop(target);
        }
        (None, true) => {
            let mut target = StdoutTarget(String::new());
            format_node(&root, &settings, &mut target)?;
            drop(target);
        }
        (None, false) => {
            let temp_path = Path::new("typstfmt.typ-temp");
            let file =
                File::create(temp_path).map_err(|_| "could not create temporary file")?;
            let mut target = FileTarget(BufWriter::new(file));
            format_node(&root, &settings, &mut target)?;
            drop(target);
            fs::rename(temp_path, input_name)
                .map_err(|_| "could not replace input file")?;
        }
    };
    Ok(())
}

pub struct FileTarget(BufWriter<File>);

impl OutputTarget for FileTarget {
    fn emit(&mut self, data: &EcoString, _settings: &settings::Settings) {
        self.0.write_all(data.as_bytes()).unwrap();
    }
}

pub struct StdoutTarget(String);

impl OutputTarget for StdoutTarget {
    fn emit(&mut self, data: &EcoString, _settings: &Settings) {
        self.0.push_str(data);
        if self.0.len() > 20 {
            print!("{}", self.0);
            self.0.clear();
        }
    }
}

impl Drop for StdoutTarget {
    fn drop(&mut self) {
        print!("{}", self.0);
    }
}
