use ecow::{EcoString, eco_format};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use typst_utils::Rdedup;

use crate::diag::{Hint, HintedStrResult};
use crate::foundations::{Dict, Fold, IntoValue, Repr, cast};
use crate::layout::Abs;
use crate::text::{FontStyle, FontVariant, Tag};

/// A variation axis in a font.
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct FontAxis {
    /// The OpenType tag that identifies the axis. May be a standard tag like
    /// `wght` or a custom tag.
    pub tag: Tag,
    /// The minimum value the font supports for this axis.
    pub min: AxisValue,
    /// The maximum value the font supports for this axis.
    pub max: AxisValue,
    /// The default value the font has set for this axis.
    pub default: AxisValue,
}

impl FontAxis {
    /// Determines the distance from the `target` value to the closest value
    /// that lies within the axis' range.
    pub(super) fn distance<T, N>(
        &self,
        target: T,
        parse: impl Fn(AxisValue) -> T,
        distance: impl Fn(T, T) -> N,
    ) -> N
    where
        T: Ord,
        N: Default,
    {
        let min = parse(self.min);
        let max = parse(self.max);
        if target < min {
            distance(min, target)
        } else if target < max {
            N::default()
        } else {
            distance(target, max)
        }
    }
}

/// A value for an OpenType font variation.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AxisValue(pub f32);

impl AxisValue {
    /// Clamps this value into the allowed range for the given `axis`.
    pub fn clamp(self, axis: &FontAxis) -> Self {
        AxisValue(self.0.clamp(axis.min.0, axis.max.0))
    }
}

impl Hash for AxisValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl Display for AxisValue {
    // Per Google Fonts Axis Registry, generally no more than two decimal digits
    // of precision are expected; we replicate std::fmt's rounding to compute
    // the appropriate amount, avoiding visual noise of trailing zeros.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let rounded_hundredths = (self.0 * 100.0).round_ties_even();
        let decimal_digits = rounded_hundredths.rem_euclid(100.) as u8;

        let precision = match decimal_digits {
            0 => 0,
            v if v % 10 == 0 => 1,
            _ => 2,
        };

        write!(f, "{:.precision$}", self.0)
    }
}

cast! {
    AxisValue,
    self => (self.0 as f64).into_value(),
    v: f64 => Self(v as f32),
}

/// Well-known variation axes that are used for font selection and/or affected
/// by text properties during instantiation.
#[derive(Default, Copy, Clone)]
pub struct StandardAxes<'a> {
    pub ital: Option<&'a FontAxis>,
    pub slnt: Option<&'a FontAxis>,
    pub wght: Option<&'a FontAxis>,
    pub wdth: Option<&'a FontAxis>,
    pub opsz: Option<&'a FontAxis>,
}

impl<'a> StandardAxes<'a> {
    pub const ITAL: Tag = Tag::from_bytes(b"ital");
    pub const SLNT: Tag = Tag::from_bytes(b"slnt");
    pub const WGHT: Tag = Tag::from_bytes(b"wght");
    pub const WDTH: Tag = Tag::from_bytes(b"wdth");
    pub const OPSZ: Tag = Tag::from_bytes(b"opsz");

    pub const LIST: [Tag; 5] =
        [Self::ITAL, Self::SLNT, Self::WGHT, Self::WDTH, Self::OPSZ];

    /// Extracts the standard axes from the given axes.
    pub fn parse(axes: &'a [FontAxis]) -> Self {
        let mut this = StandardAxes::default();
        for axis in axes {
            match axis.tag {
                Self::ITAL => this.ital = Some(axis),
                Self::SLNT => this.slnt = Some(axis),
                Self::WGHT => this.wght = Some(axis),
                Self::WDTH => this.wdth = Some(axis),
                Self::OPSZ => this.opsz = Some(axis),
                _ => {}
            }
        }
        this
    }

    /// Whether the given tag is one of the standard ones.
    pub fn knows(tag: Tag) -> bool {
        Self::LIST.contains(&tag)
    }

    /// Returns a metric with which axes can be sorted for user-facing display.
    pub fn order(tag: Tag) -> impl Ord {
        Self::LIST.iter().position(|&t| t == tag).unwrap_or(Self::LIST.len())
    }
}

/// Variable font axis values.
///
/// This stores axis tag to value mappings for variable fonts. Unlike font
/// features which are integers, axis values are floating-point numbers.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct FontVariations(pub SmallVec<[(Tag, AxisValue); 2]>);

impl FontVariations {
    /// Resolves which variations to set given the `axes` supported by a font
    /// and a desired font `variant` and point `size`.
    pub fn resolve(axes: &[FontAxis], variant: FontVariant, size: Abs) -> Self {
        let mut variations = FontVariations::default();
        let mut set = |axis: &FontAxis, value: AxisValue| {
            variations.0.push((axis.tag, value.clamp(axis)));
        };

        let axes = StandardAxes::parse(axes);

        match (variant.style, axes.ital, axes.slnt) {
            // Don't need to do anything or can't do anything.
            (FontStyle::Normal, ..) | (_, None, None) => {}

            // Serve italic due to request or as fallback for oblique.
            (FontStyle::Italic, Some(axis), _)
            | (FontStyle::Oblique, Some(axis), None) => {
                // Set to 1.0 for italic, but avoid exceeding the axis' range.
                set(axis, AxisValue(axis.max.0.min(1.0)));
            }

            // Serve oblique due to request or as fallback for italic.
            (FontStyle::Oblique, _, Some(axis))
            | (FontStyle::Italic, None, Some(axis)) => {
                // Slant values are clockwise and a typical italic is
                // counter-clockwise, so negative values are desirable. If the
                // axis doesn't support negative slant, however, we prefer,
                // positive slant over no slant at all.
                if axis.min.0 < 0.0 {
                    set(axis, axis.min);
                } else if axis.max.0 > 0.0 {
                    set(axis, axis.max);
                }
            }
        }

        if let Some(axis) = axes.wdth {
            set(axis, variant.stretch.to_wdth());
        }

        if let Some(axis) = axes.wght {
            set(axis, variant.weight.to_wght());
        }

        if let Some(axis) = axes.opsz {
            set(axis, AxisValue(size.to_pt() as f32));
        }

        variations
    }

    /// Adds additional font variations to the end.
    pub fn chain(mut self, other: &FontVariations) -> Self {
        self.0.extend_from_slice(&other.0);
        self
    }

    /// Sorts and deduplicates variations so that we have one canonical
    /// font instance for each combination.
    pub fn normalized(mut self) -> Self {
        // Sort should be stable so that later values consistently win. The
        // stable std sort only allocates for larger arrays, so this is fine.
        self.0.sort_by_key(|&(tag, _)| tag);

        // We want later values to win, so we can't use the built-in
        // `dedup_by_key` (which would let earlier ones win).
        self.0.rdedup_by_key(|&mut (tag, _)| tag);

        self
    }
}

impl Fold for FontVariations {
    fn fold(self, outer: Self) -> Self {
        Self(self.0.fold(outer.0))
    }
}

cast! {
    FontVariations,
    self => self.0
        .into_iter()
        .map(|(tag, num)|(tag.to_str_lossy().into(), num.into_value()))
        .collect::<Dict>()
        .into_value(),
    values: Dict => Self(values
        .into_iter()
        .enumerate()
        .map(|(i, (k, v))| Ok((
            k.clone().into_value().cast::<Tag>().hint(tag_hint_helper(i, &k))?,
            v.cast::<AxisValue>().hint(tag_hint_helper(i, &k))?
        )))
        .collect::<HintedStrResult<_>>()?),
}

fn tag_hint_helper(index: usize, key: &impl Repr) -> EcoString {
    eco_format!("occurred in tag at index {index} (`{}`)", key.repr())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_axis_value_fmt() {
        assert_eq!(format!("{}", AxisValue(100.)), "100");
        assert_eq!(format!("{}", AxisValue(-2.5)), "-2.5");
        assert_eq!(format!("{}", AxisValue(8.250023)), "8.25");
        assert_eq!(format!("{}", AxisValue(25_000.248)), "25000.25");
        assert_eq!(format!("{}", AxisValue(f32::NAN)), "NaN");
        assert_eq!(format!("{}", AxisValue(f32::INFINITY)), "inf");
    }
}
