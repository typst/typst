use std::{fmt::Display, str::FromStr};

use typst::ExportTarget;

bitflags::bitflags! {
    /// Bit flags for [`ExportTarget`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ExportTargets: u8 {
        /// The pdf export target.
        const PDF = 1 << 0;
        /// The SVG export target.
        const SVG = 1 << 1;
        /// The raster export target.
        const RASTER = 1 << 2;
    }
}

impl From<ExportTarget> for ExportTargets {
    fn from(value: ExportTarget) -> Self {
        match value {
            ExportTarget::Pdf => Self::PDF,
            ExportTarget::Svg => Self::SVG,
            ExportTarget::Raster => Self::RASTER,
            _ => todo!(),
        }
    }
}

impl TryFrom<ExportTargets> for ExportTarget {
    type Error = ();

    fn try_from(value: ExportTargets) -> Result<Self, Self::Error> {
        Ok(match value {
            x if x == ExportTargets::PDF => Self::Pdf,
            x if x == ExportTargets::SVG => Self::Svg,
            x if x == ExportTargets::RASTER => Self::Raster,
            _ => return Err(()),
        })
    }
}

impl ExportTargets {
    /// Turns self into an iterator of [`ExportTarget`], yielding one target for
    /// each set bit.
    pub fn iter_target(self) -> impl Iterator<Item = ExportTarget> {
        self.iter_names().map(|(_, targets)| {
            targets.try_into().expect("iter_names ensures unqiue bits")
        })
    }
}

impl FromStr for ExportTargets {
    type Err = bitflags::parser::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        bitflags::parser::from_str(s)
    }
}

impl Display for ExportTargets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}
