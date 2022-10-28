use super::{Smart, StyleChain};
use crate::geom::{Abs, Axes, Corners, Em, Length, Numeric, Rel, Sides};
use crate::library::text::TextNode;

/// A property that is resolved with other properties from the style chain.
pub trait Resolve {
    /// The type of the resolved output.
    type Output;

    /// Resolve the value using the style chain.
    fn resolve(self, styles: StyleChain) -> Self::Output;
}

impl Resolve for Em {
    type Output = Abs;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        if self.is_zero() {
            Abs::zero()
        } else {
            self.at(styles.get(TextNode::SIZE))
        }
    }
}

impl Resolve for Length {
    type Output = Abs;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.abs + self.em.resolve(styles)
    }
}

impl<T: Resolve> Resolve for Option<T> {
    type Output = Option<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Smart<T> {
    type Output = Smart<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Axes<T> {
    type Output = Axes<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Sides<T> {
    type Output = Sides<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Corners<T> {
    type Output = Corners<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T> Resolve for Rel<T>
where
    T: Resolve + Numeric,
    <T as Resolve>::Output: Numeric,
{
    type Output = Rel<<T as Resolve>::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|abs| abs.resolve(styles))
    }
}
