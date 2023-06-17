use tiny_skia::Transform;
use typst::geom::{Abs, Color};

use super::ir;

/// Additional methods for [`Length`].
pub trait AbsExt {
    /// Convert to a number of points as f32.
    fn to_f32(self) -> f32;
}

impl AbsExt for Abs {
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
    }
}

/// Additional methods for types that can be converted to CSS.
pub trait ToCssExt {
    fn to_css(self) -> String;
}

impl ToCssExt for Color {
    fn to_css(self) -> String {
        let color = self.to_rgba();
        if color.a == 255 {
            let shorter = format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
            if shorter.chars().nth(1) == shorter.chars().nth(2)
                && shorter.chars().nth(3) == shorter.chars().nth(4)
                && shorter.chars().nth(5) == shorter.chars().nth(6)
            {
                return format!(
                    "#{}{}{}",
                    shorter.chars().nth(1).unwrap(),
                    shorter.chars().nth(3).unwrap(),
                    shorter.chars().nth(5).unwrap()
                );
            }
            return shorter;
        }

        format!("#{:02x}{:02x}{:02x}{:02x}", color.r, color.g, color.b, color.a)
    }
}

impl ToCssExt for Transform {
    fn to_css(self) -> String {
        format!(
            r#"matrix({},{},{},{},{},{})"#,
            self.sx, self.ky, self.kx, self.sy, self.tx, self.ty
        )
    }
}

impl ToCssExt for ir::Transform {
    fn to_css(self) -> String {
        format!(
            r#"matrix({},{},{},{},{},{})"#,
            self.sx.0, self.ky.0, self.kx.0, self.sy.0, self.tx.0, self.ty.0
        )
    }
}
