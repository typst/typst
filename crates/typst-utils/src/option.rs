/// Extra methods for [`Option`].
pub trait OptionExt<T> {
    /// Maps an `Option<T>` to `U` by applying a function to a contained value
    /// (if `Some`) or returns a default (if `None`).
    fn map_or_default<U: Default, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U;
}

impl<T> OptionExt<T> for Option<T> {
    fn map_or_default<U: Default, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Some(x) => f(x),
            None => U::default(),
        }
    }
}

/// Check if the [`Option`]-wrapped L is same to R.
pub fn option_eq<L, R>(left: Option<L>, other: R) -> bool
where
    L: PartialEq<R>,
{
    left.is_some_and(|v| v == other)
}
