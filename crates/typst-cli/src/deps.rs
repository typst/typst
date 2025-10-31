use std::ffi::OsString;
use std::io::{self, Write};

use serde::Serialize;

use crate::args::{DepsFormat, Output};
use crate::world::SystemWorld;

/// Writes dependencies in the given format.
pub fn write_deps(
    world: &mut SystemWorld,
    dest: &Output,
    format: DepsFormat,
    outputs: Option<&[Output]>,
) -> io::Result<()> {
    match format {
        DepsFormat::Json => write_deps_json(world, dest, outputs)?,
        DepsFormat::Zero => write_deps_zero(world, dest)?,
        DepsFormat::Make => {
            if let Some(outputs) = outputs {
                write_deps_make(world, dest, outputs)?;
            }
        }
    }
    Ok(())
}

/// Writes dependencies in JSON format.
fn write_deps_json(
    world: &mut SystemWorld,
    dest: &Output,
    outputs: Option<&[Output]>,
) -> io::Result<()> {
    let decode = |dep: OsString, kind| {
        dep.into_string().map_err(|dep| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{kind} {dep:?} is not valid utf-8"),
            )
        })
    };

    let inputs = relative_dependencies(world)?
        .map(|dep| decode(dep, "input"))
        .collect::<Result<Vec<_>, _>>()?;

    let outputs = outputs
        .map(|outputs| {
            outputs
                .iter()
                .filter_map(|output| {
                    match output {
                        Output::Path(path) => {
                            Some(decode(path.clone().into_os_string(), "output"))
                        }
                        // Skip stdout
                        Output::Stdout => None,
                    }
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    #[derive(Serialize)]
    struct Deps {
        inputs: Vec<String>,
        outputs: Option<Vec<String>>,
    }

    serde_json::to_writer(dest.open()?, &Deps { inputs, outputs })?;

    Ok(())
}

/// Writes dependencies in the Zero / Text0 format.
fn write_deps_zero(world: &mut SystemWorld, dest: &Output) -> io::Result<()> {
    let mut dest = dest.open()?;
    for dep in relative_dependencies(world)? {
        dest.write_all(dep.as_encoded_bytes())?;
        dest.write_all(b"\0")?;
    }
    Ok(())
}

/// Writes dependencies in the Make format.
fn write_deps_make(
    world: &mut SystemWorld,
    dest: &Output,
    outputs: &[Output],
) -> io::Result<()> {
    let mut buffer = Vec::new();
    for (i, output) in outputs.iter().enumerate() {
        let path = match output {
            Output::Path(path) => path.as_os_str(),
            Output::Stdout => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "make dependencies contain the output path, \
                     but the output was stdout",
                ));
            }
        };

        // Silently skip paths that aren't valid Unicode so we still
        // produce a rule that will work for the other paths that can be
        // processed.
        let Some(string) = path.to_str() else { continue };
        if i != 0 {
            buffer.write_all(b" ")?;
        }
        buffer.write_all(munge(string).as_bytes())?;
    }

    // Only create the deps file in case of valid output paths.
    let mut dest = dest.open()?;
    dest.write_all(&buffer)?;
    dest.write_all(b":")?;

    for dep in relative_dependencies(world)? {
        // See above.
        let Some(string) = dep.to_str() else { continue };
        dest.write_all(b" ")?;
        dest.write_all(munge(string).as_bytes())?;
    }
    dest.write_all(b"\n")?;

    Ok(())
}

// Based on `munge` in libcpp/mkdeps.cc from the GCC source code. This isn't
// perfect as some special characters can't be escaped.
fn munge(s: &str) -> String {
    let mut res = String::with_capacity(s.len());
    let mut slashes = 0;
    for c in s.chars() {
        match c {
            '\\' => slashes += 1,
            '$' => {
                res.push('$');
                slashes = 0;
            }
            ':' => {
                res.push('\\');
                slashes = 0;
            }
            ' ' | '\t' => {
                // `munge`'s source contains a comment here that says: "A
                // space or tab preceded by 2N+1 backslashes represents N
                // backslashes followed by space..."
                for _ in 0..slashes + 1 {
                    res.push('\\');
                }
                slashes = 0;
            }
            '#' => {
                res.push('\\');
                slashes = 0;
            }
            _ => slashes = 0,
        };
        res.push(c);
    }
    res
}

/// Extracts the current compilation's dependencies as paths relative to the
/// current directory.
fn relative_dependencies(
    world: &mut SystemWorld,
) -> io::Result<impl Iterator<Item = OsString>> {
    let root = world.root().to_owned();
    let current_dir = std::env::current_dir()?;
    let relative_root =
        pathdiff::diff_paths(&root, &current_dir).unwrap_or_else(|| root.clone());
    Ok(world.dependencies().map(move |dependency| {
        dependency
            .strip_prefix(&root)
            .map_or_else(|_| dependency.clone(), |x| relative_root.join(x))
            .into_os_string()
    }))
}
