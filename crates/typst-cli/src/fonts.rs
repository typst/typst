use std::any::Any;
use std::fmt::{self, Display, Formatter};
use std::path::Path;

use typst::text::{
    AxisValue, FontAxis, FontInfo, FontStretch, FontVariant, FontWeight, StandardAxes,
};
use typst_kit::fonts::{self, FontPath, FontStore};

use crate::args::{FontArgs, FontsCommand};

/// Execute a font listing command, marking shadowed variants.
pub fn fonts(command: &FontsCommand) {
    let fonts = discover_fonts(&command.font);
    let book = fonts.book();

    for (family, indices) in book.families() {
        println!("{family}");
        if command.variants {
            let indices: Vec<_> = indices.collect();
            for (i, &index) in indices.iter().enumerate() {
                let info = book.info(index).unwrap();
                let shadowed_by = indices[..i]
                    .iter()
                    .find(|&&other| {
                        let other = book.info(other).unwrap();
                        other.variant == info.variant
                            && other.axes.is_empty()
                            && info.axes.is_empty()
                    })
                    .map(|&other| font_path(&fonts, other));
                let path = font_path(&fonts, index);
                let last = i + 1 == indices.len();
                let variant = typst_utils::display(|f| {
                    write_variant(f, info, path, shadowed_by, last)
                });
                print!("{variant}");
            }
            println!();
        }
    }
}

/// Retrieves the path of a font, or `None` if it is embedded.
fn font_path(fonts: &FontStore, index: usize) -> Option<&Path> {
    fonts
        .source(index)
        .and_then(|source| (source as &dyn Any).downcast_ref::<FontPath>())
        .map(|font| font.path.as_path())
}

/// Discovers the fonts as specified by the CLI flags.
#[typst_macros::time(name = "discover fonts")]
pub fn discover_fonts(args: &FontArgs) -> FontStore {
    let mut fonts = FontStore::new();

    if !args.ignore_system_fonts {
        fonts.extend(fonts::system());
    }

    #[cfg(feature = "embedded-fonts")]
    if !args.ignore_embedded_fonts {
        fonts.extend(fonts::embedded());
    }

    for path in args.font_paths.iter().flatten() {
        fonts.extend(fonts::scan(path));
    }

    fonts
}

/// Displays information for one font file / variant.
fn write_variant(
    f: &mut Formatter,
    info: &FontInfo,
    path: Option<&Path>,
    shadowed_by: Option<Option<&Path>>,
    last: bool,
) -> fmt::Result {
    let path = display_path(path);

    let FontVariant { style, weight, stretch } = info.variant;
    let marker = if last { '└' } else { '├' };
    let pad = if last { "     " } else { "  │  " };

    let mut axes = info.axes.clone();
    axes.sort_by_key(|axis| StandardAxes::order(axis.tag));

    if axes.is_empty() {
        writeln!(f, "  {marker} {path}")?;
        writeln!(f, "{pad} Style: {style:?}, Weight: {weight}, Stretch: {stretch}")?;
    } else {
        writeln!(f, "  {marker} {path} (Variable)")?;
        let standard = StandardAxes::parse(&axes);
        if standard.ital.is_none() && standard.slnt.is_none() {
            writeln!(f, "{pad} Style: {style:?}")?;
        }
        if standard.wght.is_none() {
            writeln!(f, "{pad} Weight: {weight}")?;
        }
        if standard.wdth.is_none() {
            writeln!(f, "{pad} Stretch: {stretch}")?;
        }
        for axis in &axes {
            writeln!(f, "{pad} {}", typst_utils::display(|f| write_axis(f, axis)))?;
        }
    }

    if let Some(other) = shadowed_by {
        writeln!(f, "{pad} ⚠ Shadowed by: {} (same variant)", display_path(other))?;
    }

    Ok(())
}

/// Displays a font's path, or that it is embedded.
fn display_path(path: Option<&Path>) -> impl Display {
    typst_utils::display(move |f| match path {
        Some(path) => path.display().fmt(f),
        None => f.pad("(Embedded)"),
    })
}

/// Formats a variation axis.
fn write_axis(f: &mut Formatter, axis: &FontAxis) -> fmt::Result {
    use std::convert::identity;
    match axis.tag {
        StandardAxes::ITAL => write_axis_with(f, axis, "Italic", identity),
        StandardAxes::SLNT => write_axis_with(f, axis, "Slant", identity),
        StandardAxes::WGHT => write_axis_with(f, axis, "Weight", FontWeight::from_wght),
        StandardAxes::WDTH => write_axis_with(f, axis, "Stretch", FontStretch::from_wdth),
        StandardAxes::OPSZ => write_axis_with(f, axis, "Optical Size", |v| {
            typst_utils::display(move |f| write!(f, "{v}pt"))
        }),
        _ => write_axis_with(f, axis, &axis.tag.to_str_lossy(), identity),
    }
}

/// Formats a variation axis with a specific name and value display function.
fn write_axis_with<T: Display>(
    f: &mut Formatter,
    axis: &FontAxis,
    name: &str,
    show: impl Fn(AxisValue) -> T,
) -> fmt::Result {
    write!(
        f,
        "{name}: {}-{} (Default: {})",
        show(axis.min),
        show(axis.max),
        show(axis.default),
    )
}
