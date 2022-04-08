use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg};

use super::{Resolve, StyleChain};
use crate::geom::{Align, Em, Length, Numeric, Relative, SpecAxis};
use crate::library::text::{ParNode, TextNode};

/// The unresolved alignment representation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawAlign {
    /// Align at the start side of the text direction.
    Start,
    /// Align at the end side of the text direction.
    End,
    /// Align at a specific alignment.
    Specific(Align),
}

impl Resolve for RawAlign {
    type Output = Align;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        let dir = styles.get(ParNode::DIR);
        match self {
            Self::Start => dir.start().into(),
            Self::End => dir.end().into(),
            Self::Specific(align) => align,
        }
    }
}

impl RawAlign {
    /// The axis this alignment belongs to.
    pub const fn axis(self) -> SpecAxis {
        match self {
            Self::Start | Self::End => SpecAxis::Horizontal,
            Self::Specific(align) => align.axis(),
        }
    }
}

impl Debug for RawAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start => f.pad("left"),
            Self::End => f.pad("center"),
            Self::Specific(align) => align.fmt(f),
        }
    }
}

/// The unresolved length representation.
///
/// Currently supports absolute and em units, but support could quite easily be
/// extended to other units that can be resolved through a style chain.
/// Probably, it would be a good idea to then move to an enum representation
/// that has a small footprint and allocates for the rare case that units are
/// mixed.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RawLength {
    /// The absolute part.
    pub length: Length,
    /// The font-relative part.
    pub em: Em,
}

impl RawLength {
    /// The zero length.
    pub const fn zero() -> Self {
        Self { length: Length::zero(), em: Em::zero() }
    }
}

impl Debug for RawLength {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match (self.length.is_zero(), self.em.is_zero()) {
            (false, false) => write!(f, "{:?} + {:?}", self.length, self.em),
            (true, false) => self.em.fmt(f),
            (_, true) => self.length.fmt(f),
        }
    }
}

impl Resolve for Em {
    type Output = Length;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        if self.is_zero() {
            Length::zero()
        } else {
            self.at(styles.get(TextNode::SIZE))
        }
    }
}

impl Resolve for RawLength {
    type Output = Length;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.length + self.em.resolve(styles)
    }
}

impl From<Length> for RawLength {
    fn from(length: Length) -> Self {
        Self { length, em: Em::zero() }
    }
}

impl From<Em> for RawLength {
    fn from(em: Em) -> Self {
        Self { length: Length::zero(), em }
    }
}

impl From<Length> for Relative<RawLength> {
    fn from(length: Length) -> Self {
        Relative::from(RawLength::from(length))
    }
}

impl Numeric for RawLength {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.length.is_finite() && self.em.is_finite()
    }
}

impl Neg for RawLength {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self { length: -self.length, em: -self.em }
    }
}

impl Add for RawLength {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            length: self.length + rhs.length,
            em: self.em + rhs.em,
        }
    }
}

sub_impl!(RawLength - RawLength -> RawLength);

impl Mul<f64> for RawLength {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            length: self.length * rhs,
            em: self.em * rhs,
        }
    }
}

impl Div<f64> for RawLength {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            length: self.length / rhs,
            em: self.em / rhs,
        }
    }
}

assign_impl!(RawLength += RawLength);
assign_impl!(RawLength -= RawLength);
assign_impl!(RawLength *= f64);
assign_impl!(RawLength /= f64);
