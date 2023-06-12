use std::any::TypeId;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use once_cell::sync::Lazy;

use super::{Content, Selector, Styles};
use crate::diag::SourceResult;
use crate::eval::{cast, Args, Dict, Func, FuncInfo, Value, Vm};

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
    fn construct(vm: &mut Vm, args: &mut Args) -> SourceResult<Content>;
}

/// An element's set rule.
pub trait Set {
    /// Parse relevant arguments into style properties for this element.
    fn set(args: &mut Args) -> SourceResult<Styles>;
}

/// An element's function.
#[derive(Copy, Clone)]
pub struct ElemFunc(pub(super) &'static NativeElemFunc);

impl ElemFunc {
    /// The function's name.
    pub fn name(self) -> &'static str {
        self.0.name
    }

    /// Apply the given arguments to the function.
    pub fn with(self, args: Args) -> Func {
        Func::from(self).with(args)
    }

    /// Extract details about the function.
    pub fn info(&self) -> &'static FuncInfo {
        &self.0.info
    }

    /// Construct an element.
    pub fn construct(self, vm: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        (self.0.construct)(vm, args)
    }

    /// Whether the contained element has the given capability.
    pub fn can<C>(&self) -> bool
    where
        C: ?Sized + 'static,
    {
        (self.0.vtable)(TypeId::of::<C>()).is_some()
    }

    /// Create a selector for elements of this function.
    pub fn select(self) -> Selector {
        Selector::Elem(self, None)
    }

    /// Create a selector for elements of this function, filtering for those
    /// whose [fields](super::Content::field) match the given arguments.
    pub fn where_(self, fields: Dict) -> Selector {
        Selector::Elem(self, Some(fields))
    }

    /// Execute the set rule for the element and return the resulting style map.
    pub fn set(self, mut args: Args) -> SourceResult<Styles> {
        let styles = (self.0.set)(&mut args)?;
        args.finish()?;
        Ok(styles)
    }
}

impl Debug for ElemFunc {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.name())
    }
}

impl Eq for ElemFunc {}

impl PartialEq for ElemFunc {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Hash for ElemFunc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as *const _ as usize);
    }
}

cast! {
    ElemFunc,
    self => Value::Func(self.into()),
    v: Func => v.element().ok_or("expected element function")?,
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
    pub construct: fn(&mut Vm, &mut Args) -> SourceResult<Content>,
    /// The element's set rule.
    pub set: fn(&mut Args) -> SourceResult<Styles>,
    /// Details about the function.
    pub info: Lazy<FuncInfo>,
}
