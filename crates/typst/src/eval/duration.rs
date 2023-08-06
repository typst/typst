use typst_macros::cast;

/// A duration object that represents either a positive or negative span of time.
#[derive(Clone, Debug, Copy, PartialEq, Hash)]
pub struct Duration(time::Duration);

impl From<time::Duration> for Duration {
    fn from(value: time::Duration) -> Self {
        Self(value)
    }
}

cast! {
    type Duration: "duration",
}
