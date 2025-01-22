use std::{
    fs::{create_dir, create_dir_all},
    path::PathBuf,
};

use ecow::eco_format;
use typst::{
    diag::{bail, HintedStrResult, Warned},
    layout::PagedDocument,
};
use typst_kit::package::{DEFAULT_PACKAGES_SUBDIR, DEFAULT_VENDOR_SUBDIR};

use crate::{
    args::VendorCommand, compile::print_diagnostics, set_failed, world::SystemWorld,
};
use typst::World;

/// Execute a vendor command.
pub fn vendor(command: &VendorCommand) -> HintedStrResult<()> {
    let mut world = SystemWorld::new(&command.input, &command.world, &command.process)?;

    // Reset everything and ensure that the main file is present.
    world.reset();
    world.source(world.main()).map_err(|err| err.to_string())?;

    let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);

    match output {
        Ok(_) => {
            copy_deps(&mut world, &command.world.package.vendor_path)?;
            print_diagnostics(&world, &[], &warnings, command.process.diagnostic_format)
                .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();
            print_diagnostics(
                &world,
                &errors,
                &warnings,
                command.process.diagnostic_format,
            )
            .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }
    }

    Ok(())
}

fn copy_deps(
    world: &mut SystemWorld,
    vendor_path: &Option<PathBuf>,
) -> HintedStrResult<()> {
    let vendor_dir = match vendor_path {
        Some(path) => match path.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    if let Err(err) = create_dir(path) {
                        bail!("failed to create vendor directory: {:?}", err);
                    }
                    path.clone()
                } else {
                    bail!("failed to canonicalize vendor directory path: {:?}", err);
                }
            }
        },
        None => world.workdir().join(DEFAULT_VENDOR_SUBDIR),
    };

    // Must iterate two times in total. As soon as the parent directory is created,
    // world tries to read the subsequent files from the same package
    // from the vendor directory since it is higher priority.
    let all_deps = world
        .dependencies()
        .filter_map(|dep_path| {
            let path = dep_path.to_str().unwrap();
            path.find(DEFAULT_PACKAGES_SUBDIR).map(|pos| {
                let dependency_path = &path[pos + DEFAULT_PACKAGES_SUBDIR.len() + 1..];
                (dep_path.clone(), vendor_dir.join(dependency_path))
            })
        })
        .collect::<Vec<_>>();

    for (from_data_path, to_vendor_path) in all_deps {
        if let Some(parent) = to_vendor_path.parent() {
            match parent.try_exists() {
                Ok(false) => {
                    if let Err(err) = create_dir_all(parent) {
                        bail!(
                            "failed to create package inside the vendor directory: {:?}",
                            err
                        );
                    }
                }
                Err(err) => {
                    bail!("failed to check existence of a package inside the vendor directory: {:?}", err);
                }
                _ => {}
            }
        }

        if let Err(err) = std::fs::copy(from_data_path, to_vendor_path) {
            bail!("failed to copy dependency to vendor directory: {:?}", err);
        }
    }
    Ok(())
}
