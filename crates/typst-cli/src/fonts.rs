use std::any::Any;
use std::fmt::{self, Display, Formatter};
use std::path::Path;

use typst::text::{
    AxisValue, FontAxis, FontInfo, FontStretch, FontVariant, FontWeight, StandardAxes,
};
use typst_kit::fonts::{self, FontPath, FontStore};

use crate::args::{FontArgs, FontsCommand};

/// Execute a font listing command.
pub fn fonts(command: &FontsCommand) {
    let fonts = discover_fonts(&command.font);

    for (family, indices) in fonts.book().families() {
        println!("{family}");
        if command.variants {
            let mut indices = indices.peekable();
            while let Some(index) = indices.next() {
                let info = fonts.book().info(index).unwrap();
                let path = fonts
                    .source(index)
                    .and_then(|source| (source as &dyn Any).downcast_ref::<FontPath>())
                    .map(|font| font.path.as_path());
                let last = indices.peek().is_none();
                let variant =
                    typst_utils::display(|f| write_variant(f, info, path, last));
                print!("{variant}");
            }
            println!();
        }
    }
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

    for path in &args.font_paths {
        fonts.extend(fonts::scan(path));
    }

    fonts
}

/// Displays information for one font file / variant.
fn write_variant(
    f: &mut Formatter,
    info: &FontInfo,
    path: Option<&Path>,
    last: bool,
) -> fmt::Result {
    let path = typst_utils::display(|f| match path {
        Some(path) => path.display().fmt(f),
        None => f.pad("(Embedded)"),
    });

    let FontVariant { style, weight, stretch } = info.variant;
    let marker = if last { '└' } else { '├' };
    let pad = if last { "     " } else { "  │  " };

    let mut axes = info.axes.to_vec();
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

    Ok(())
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
