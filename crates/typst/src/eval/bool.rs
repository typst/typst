use super::ty;

/// A type with two states.
///
/// The boolean type has two values: `{true}` and `{false}`. It denotes whether
/// something is active or enabled.
///
/// # Example
/// ```example
/// #false \
/// #true \
/// #(1 < 2)
/// ```
#[ty(title = "Boolean")]
type bool;
