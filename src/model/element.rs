use std::any::TypeId;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use ecow::EcoString;
use once_cell::sync::Lazy;

use super::{Content, Selector, Styles};
use crate::diag::SourceResult;
use crate::eval::{
    cast_from_value, cast_to_value, Args, Dict, Func, FuncInfo, Value, Vm,
};

/// A document element.
pub trait Element: Construct + Set + Sized + 'static {
    /// Pack the element into type-erased content.
    fn pack(self) -> Content;

    /// Extract this element from type-erased content.
    fn unpack(content: &Content) -> Option<&Self>;

    /// The element's function.
    fn func() -> ElemFunc;
}

/// An element's constructor function.
pub trait Construct {
    /// Construct an element from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// element's set rule.
    #[allow(clippy::missing_errors_doc /* obvious */)]
    fn construct(vm: &mut Vm<'_>, args: &mut Args) -> SourceResult<Content>;
}

/// An element's set rule.
pub trait Set {
    /// Parse relevant arguments into style properties for this element.
    ///
    /// # Errors
    ///
    /// If the arguments cannot be parsed into the element's settable properties.
    fn set(args: &mut Args) -> SourceResult<Styles>;
}

/// An element's function.
#[derive(Copy, Clone)]
pub struct ElemFunc(pub(super) &'static NativeElemFunc);

impl ElemFunc {
    /// The function's name.
    #[inline]
    #[must_use]
    pub fn name(self) -> &'static str {
        self.0.name
    }

    /// Apply the given arguments to the function.
    #[inline]
    #[must_use]
    pub fn with(self, args: Args) -> Func {
        Func::from(self).with(args)
    }

    /// Extract details about the function.
    #[inline]
    #[must_use]
    pub fn info(&self) -> &'static FuncInfo {
        &self.0.info
    }

    /// Construct an element.
    ///
    /// # Errors
    ///
    /// Directly propagated from the element's constructor.
    #[inline]
    pub fn construct(self, vm: &mut Vm<'_>, args: &mut Args) -> SourceResult<Content> {
        (self.0.construct)(vm, args)
    }

    /// Create a selector for elements of this function.
    #[inline]
    #[must_use]
    pub fn select(self) -> Selector {
        Selector::Elem(self, None)
    }

    /// Create a selector for elements of this function, filtering for those
    /// whose [fields](super::Content::field) match the given arguments.
    #[inline]
    #[must_use]
    pub fn where_(self, fields: Dict) -> Selector {
        Selector::Elem(self, Some(fields))
    }

    /// Execute the set rule for the element and return the resulting style map.
    ///
    /// # Errors
    ///
    /// Directly propagated from the element's set method, or if there are arguments left over.
    #[inline]
    pub fn set(self, mut args: Args) -> SourceResult<Styles> {
        let styles = (self.0.set)(&mut args)?;
        args.finish()?;
        Ok(styles)
    }
}

impl Debug for ElemFunc {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(self.name())
    }
}

impl Eq for ElemFunc {}

impl PartialEq for ElemFunc {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Hash for ElemFunc {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        (self.0 as *const NativeElemFunc).hash(hasher);
    }
}

cast_from_value! {
    ElemFunc,
    v: Func => v.element().ok_or("expected element function")?,
}

cast_to_value! {
    v: ElemFunc => Value::Func(v.into())
}

impl From<&'static NativeElemFunc> for ElemFunc {
    fn from(native: &'static NativeElemFunc) -> Self {
        Self(native)
    }
}

/// An element function backed by a Rust type.
pub struct NativeElemFunc {
    /// The element's name.
    pub name: &'static str,
    /// The element's vtable for capability dispatch.
    pub vtable: fn(of: TypeId) -> Option<*const ()>,
    /// The element's constructor.
    pub construct: fn(&mut Vm<'_>, &mut Args) -> SourceResult<Content>,
    /// The element's set rule.
    pub set: fn(&mut Args) -> SourceResult<Styles>,
    /// Details about the function.
    pub info: Lazy<FuncInfo>,
}

impl Debug for NativeElemFunc {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeElemFunc")
            .field("name", &self.name)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

/// A label for an element.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Label(pub EcoString);

impl Debug for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

/// Indicates that an element cannot be labelled.
pub trait Unlabellable {}
