//! Utilities for Typst.

pub mod fat;

#[macro_use]
mod macros;
mod defer;
mod fmt;
mod hash;
mod math;
mod numeric;
#[path = "version.rs"]
mod version_;

#[cfg(feature = "full")]
block! {
    mod bitset;
    mod collection;
    mod def_site;
    mod deferred;
    mod duration;
    mod get;
    mod listset;
    mod option;
    mod pico;
    mod protected;
    mod round;
    mod scalar;
    #[path = "static.rs"]
    mod static_;
}

pub use self::defer::defer;
pub use self::fmt::{debug, display};
pub use self::hash::{HashLock, LazyHash, ManuallyHash, hash128};
pub use self::math::default_math_class;
pub use self::numeric::{NonZeroExt, Numeric, NumericLength};
pub use self::version_::{TypstVersion, display_commit, version};

#[cfg(feature = "full")]
block! {
    pub use self::bitset::{BitSet, SmallBitSet};
    pub use self::collection::{GroupByKey, MaybeReverseIter, Rdedup, SliceExt};
    pub use self::def_site::DefSite;
    pub use self::deferred::Deferred;
    pub use self::duration::format_duration;
    pub use self::get::Get;
    pub use self::listset::ListSet;
    pub use self::option::{OptionExt, option_eq};
    pub use self::pico::{PicoStr, ResolvedPicoStr};
    pub use self::protected::Protected;
    pub use self::round::{round_int_with_precision, round_with_precision};
    pub use self::scalar::Scalar;
    pub use self::static_::Static;

    #[doc(hidden)]
    pub use once_cell;
}
