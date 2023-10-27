use ::std::hash::Hasher;
use ::typst::model::{Guard, Label, Location};
use ::typst::syntax::Span;
use comemo::Prehashed;
use ecow::EcoString;
use std::any::{Any, TypeId};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::sync::Arc;

use once_cell::sync::Lazy;

use super::{Content, Selector, Styles};
use crate::diag::{SourceResult, StrResult};
use crate::eval::{cast, Args, Dict, Func, ParamInfo, Repr, Scope, Value, Vm};
use crate::util::Static;

pub trait Element: Any + Send + Sync + Debug + Repr + 'static {
    fn data(&self) -> ElementData;

    fn span(&self) -> Span;

    fn set_span(&mut self, span: Span);

    fn location(&self) -> Option<Location>;

    fn set_location(&mut self, location: Location);

    fn label(&self) -> Option<&Label>;

    fn set_label(&mut self, label: Label);

    fn push_guard(&mut self, guard: Guard);

    fn is_guarded(&self, guard: Guard) -> bool;

    fn guards(&self) -> &[::typst::model::Guard];

    fn is_pristine(&self) -> bool;

    fn mark_prepared(&mut self);

    fn needs_preparation(&self) -> bool;

    fn is_prepared(&self) -> bool;

    fn dyn_hash(&self, hasher: &mut dyn Hasher);

    fn dyn_eq(&self, other: &Content) -> bool;

    fn field(&self, name: &str) -> Option<Value>;

    fn children(&self) -> &[Prehashed<Content>];

    fn dyn_clone(&self) -> Arc<dyn Element>;

    /// Get the fields of the element.
    fn fields(&self) -> Dict;

    /// Set the fields of the element.
    fn set_field(&mut self, name: &str, value: Value) -> StrResult<()>;

    fn name(&self) -> &'static str {
        self.data().name()
    }

    fn can_type_id(&self, type_id: TypeId) -> bool {
        self.data().can_type_id(type_id)
    }

    fn vtable(&self) -> fn(of: TypeId) -> Option<*const ()> {
        self.data().vtable()
    }
}

/// A document element.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ElementData(Static<NativeElementData>);

impl ElementData {
    /// Get the element for `T`.
    pub fn of<T: NativeElement>() -> Self {
        T::elem()
    }

    /// Is the element static?
    pub fn is_static(self) -> bool {
        self.0.static_
    }

    /// Create an element from the given data.
    pub fn empty(self) -> Content {
        (self.0.empty)()
    }

    /// The element's normal name (e.g. `enum`).
    pub fn name(self) -> &'static str {
        self.0.name
    }

    /// The element's title case name, for use in documentation
    /// (e.g. `Numbered List`).
    pub fn title(&self) -> &'static str {
        self.0.title
    }

    /// Documentation for the element (as Markdown).
    pub fn docs(&self) -> &'static str {
        self.0.docs
    }

    /// Search keywords for the element.
    pub fn keywords(&self) -> &'static [&'static str] {
        self.0.keywords
    }

    /// Construct an instance of this element.
    pub fn construct(self, vm: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        (self.0.construct)(vm, args)
    }

    /// Execute the set rule for the element and return the resulting style map.
    pub fn set(self, vm: &mut Vm, mut args: Args) -> SourceResult<Styles> {
        let styles = (self.0.set)(vm, &mut args)?;
        args.finish()?;
        Ok(styles)
    }

    /// Whether the element has the given capability.
    pub fn can<C>(self) -> bool
    where
        C: ?Sized + 'static,
    {
        self.can_type_id(TypeId::of::<C>())
    }

    /// Whether the element has the given capability where the capability is
    /// given by a `TypeId`.
    pub fn can_type_id(self, type_id: TypeId) -> bool {
        (self.0.vtable)(type_id).is_some()
    }

    /// The VTable for capabilities dispatch.
    pub fn vtable(self) -> fn(of: TypeId) -> Option<*const ()> {
        self.0.vtable
    }

    /// Create a selector for this element.
    pub fn select(self) -> Selector {
        Selector::Elem(self, None)
    }

    /// Create a selector for this element, filtering for those
    /// that [fields](super::Content::field) match the given argument.
    pub fn where_(self, fields: Dict) -> Selector {
        Selector::Elem(self, Some(fields))
    }

    /// The element's associated scope of sub-definition.
    pub fn scope(&self) -> &'static Scope {
        &(self.0).0.scope
    }

    /// Details about the element's fields.
    pub fn params(&self) -> &'static [ParamInfo] {
        &(self.0).0.params
    }
}

impl Repr for ElementData {
    fn repr(&self) -> EcoString {
        self.name().into()
    }
}

impl Ord for ElementData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name().cmp(other.name())
    }
}

impl PartialOrd for ElementData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

cast! {
    ElementData,
    self => Value::Func(self.into()),
    v: Func => v.element().ok_or("expected element")?,
}

/// A Typst element that is defined by a native Rust type.
pub trait NativeElement: Construct + Set + Sized + 'static {
    /// Get the element for the native Rust element.
    fn elem() -> ElementData {
        ElementData::from(Self::data())
    }

    /// Get the element data for the native Rust element.
    fn data() -> &'static NativeElementData;

    /// Pack the element into type-erased content.
    fn pack(self) -> Content;

    /// Extract this element from type-erased content.
    fn unpack_owned(content: Content) -> Option<Arc<Self>>;

    /// Extract this element from type-erased content.
    fn unpack(content: &Content) -> Option<&Self>;

    /// Extract this element from type-erased content.
    fn unpack_mut(content: &mut Content) -> Option<&mut Self>;
}

/// An element's constructor function.
pub trait Construct {
    /// The output type of the constructor.
    type Output;

    /// Construct an element from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// element's set rule.
    fn construct(vm: &mut Vm, args: &mut Args) -> SourceResult<Self::Output>;
}

/// An element's set rule.
pub trait Set {
    /// Parse relevant arguments into style properties for this element.
    fn set(vm: &mut Vm, args: &mut Args) -> SourceResult<Styles>;
}

/// Defines a native element.
#[derive(Debug)]
pub struct NativeElementData {
    pub name: &'static str,
    pub title: &'static str,
    pub docs: &'static str,
    /// Whether the element is struct-based (true) or dynamic (false).
    pub static_: bool,
    pub keywords: &'static [&'static str],
    pub empty: fn() -> Content,
    pub construct: fn(&mut Vm, &mut Args) -> SourceResult<Content>,
    pub set: fn(&mut Vm, &mut Args) -> SourceResult<Styles>,
    pub vtable: fn(of: TypeId) -> Option<*const ()>,
    pub scope: Lazy<Scope>,
    pub params: Lazy<Vec<ParamInfo>>,
}

impl From<&'static NativeElementData> for ElementData {
    fn from(data: &'static NativeElementData) -> Self {
        Self(Static(data))
    }
}

cast! {
    &'static NativeElementData,
    self => ElementData::from(self).into_value(),
}
