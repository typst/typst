use super::Smart;
use crate::geom::{Abs, Corners, Length, Rel, Sides};

/// A property that is folded to determine its final value.
pub trait Fold {
    /// The type of the folded output.
    type Output;

    /// Fold this inner value with an outer folded value.
    fn fold(self, outer: Self::Output) -> Self::Output;
}

impl<T> Fold for Option<T>
where
    T: Fold,
    T::Output: Default,
{
    type Output = Option<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.map(|inner| inner.fold(outer.unwrap_or_default()))
    }
}

impl<T> Fold for Smart<T>
where
    T: Fold,
    T::Output: Default,
{
    type Output = Smart<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.map(|inner| inner.fold(outer.unwrap_or_default()))
    }
}

impl<T> Fold for Sides<T>
where
    T: Fold,
{
    type Output = Sides<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.fold(outer))
    }
}

impl Fold for Sides<Option<Rel<Abs>>> {
    type Output = Sides<Rel<Abs>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
    }
}

impl Fold for Sides<Option<Smart<Rel<Length>>>> {
    type Output = Sides<Smart<Rel<Length>>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
    }
}

impl<T> Fold for Corners<T>
where
    T: Fold,
{
    type Output = Corners<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.fold(outer))
    }
}

impl Fold for Corners<Option<Rel<Abs>>> {
    type Output = Corners<Rel<Abs>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
    }
}
