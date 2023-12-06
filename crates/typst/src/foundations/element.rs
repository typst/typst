use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use ecow::EcoString;
use once_cell::sync::Lazy;
use smallvec::SmallVec;

use crate::diag::{SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, Args, Content, Dict, Func, Label, ParamInfo, Repr, Scope, Selector, StyleChain,
    Styles, Value,
};
use crate::introspection::Location;
use crate::syntax::Span;
use crate::text::{Lang, Region};
use crate::util::Static;

#[doc(inline)]
pub use typst_macros::elem;

/// A document element.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Element(Static<NativeElementData>);

impl Element {
    /// Get the element for `T`.
    pub fn of<T: NativeElement>() -> Self {
        T::elem()
    }

    /// Extract the field ID for the given field name.
    pub fn field_id(&self, name: &str) -> Option<u8> {
        (self.0.field_id)(name)
    }

    /// Extract the field name for the given field ID.
    pub fn field_name(&self, id: u8) -> Option<&'static str> {
        (self.0.field_name)(id)
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
    pub fn construct(
        self,
        engine: &mut Engine,
        args: &mut Args,
    ) -> SourceResult<Content> {
        (self.0.construct)(engine, args)
    }

    /// Execute the set rule for the element and return the resulting style map.
    pub fn set(self, engine: &mut Engine, mut args: Args) -> SourceResult<Styles> {
        let styles = (self.0.set)(engine, &mut args)?;
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

    /// Create a selector for this element, filtering for those that
    /// [fields](crate::foundations::Content::field) match the given argument.
    pub fn where_(self, fields: SmallVec<[(u8, Value); 1]>) -> Selector {
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

    /// The element's local name, if any.
    pub fn local_name(&self, lang: Lang, region: Option<Region>) -> Option<&'static str> {
        (self.0).0.local_name.map(|f| f(lang, region))
    }
}

impl Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Element({})", self.name())
    }
}

impl Repr for Element {
    fn repr(&self) -> EcoString {
        self.name().into()
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name().cmp(other.name())
    }
}

impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

cast! {
    Element,
    self => Value::Func(self.into()),
    v: Func => v.element().ok_or("expected element")?,
}

/// Fields of an element.
pub trait ElementFields {
    /// The fields of the element.
    type Fields;
}

/// A Typst element that is defined by a native Rust type.
pub trait NativeElement: Debug + Repr + Construct + Set + Send + Sync + 'static {
    /// Get the element for the native Rust element.
    fn elem() -> Element
    where
        Self: Sized,
    {
        Element::from(Self::data())
    }

    /// Pack the element into type-erased content.
    fn pack(self) -> Content
    where
        Self: Sized,
    {
        Content::new(self)
    }

    /// Get the element data for the native Rust element.
    fn data() -> &'static NativeElementData
    where
        Self: Sized;

    /// Get the element data for the native Rust element.
    fn dyn_elem(&self) -> Element;

    /// Dynamically hash the element.
    fn dyn_hash(&self, hasher: &mut dyn Hasher);

    /// Dynamically compare the element.
    fn dyn_eq(&self, other: &Content) -> bool;

    /// Dynamically clone the element.
    fn dyn_clone(&self) -> Arc<dyn NativeElement>;

    /// Get the element as a dynamic value.
    fn as_any(&self) -> &dyn Any;

    /// Get the element as a mutable dynamic value.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get the element as a dynamic value.
    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;

    /// Get the element's span.
    ///
    /// May be detached if it has not been set.
    fn span(&self) -> Span;

    /// Sets the span of this element.
    fn set_span(&mut self, span: Span);

    /// Set the element's span.
    fn spanned(mut self, span: Span) -> Self
    where
        Self: Sized,
    {
        self.set_span(span);
        self
    }

    /// Get the element's label.
    fn label(&self) -> Option<Label>;

    /// Sets the label of this element.
    fn set_label(&mut self, label: Label);

    /// Set the element's label.
    fn labelled(mut self, label: Label) -> Self
    where
        Self: Sized,
    {
        self.set_label(label);
        self
    }

    /// Get the element's location.
    fn location(&self) -> Option<Location>;

    /// Sets the location of this element.
    fn set_location(&mut self, location: Location);

    /// Checks whether the element is guarded by the given guard.
    fn is_guarded(&self, guard: Guard) -> bool;

    /// Pushes a guard onto the element.
    fn push_guard(&mut self, guard: Guard);

    /// Whether the element is pristine.
    fn is_pristine(&self) -> bool;

    /// Mark the element as having been prepared.
    fn mark_prepared(&mut self);

    /// Whether this element needs preparations.
    fn needs_preparation(&self) -> bool;

    /// Whether this element has been prepared.
    fn is_prepared(&self) -> bool;

    /// Get the field with the given field ID.
    fn field(&self, id: u8) -> Option<Value>;

    /// Whether the element has the given field set.
    fn has(&self, id: u8) -> bool;

    /// Set the field with the given ID.
    fn set_field(&mut self, id: u8, value: Value) -> StrResult<()>;

    /// Get the fields of the element.
    fn fields(&self) -> Dict;
}

impl Hash for dyn NativeElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

/// An element's constructor function.
pub trait Construct {
    /// Construct an element from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// element's set rule.
    fn construct(engine: &mut Engine, args: &mut Args) -> SourceResult<Content>
    where
        Self: Sized;
}

/// An element's set rule.
pub trait Set {
    /// Parse relevant arguments into style properties for this element.
    fn set(engine: &mut Engine, args: &mut Args) -> SourceResult<Styles>
    where
        Self: Sized;
}

/// Defines a native element.
#[derive(Debug)]
pub struct NativeElementData {
    pub name: &'static str,
    pub title: &'static str,
    pub docs: &'static str,
    pub keywords: &'static [&'static str],
    pub construct: fn(&mut Engine, &mut Args) -> SourceResult<Content>,
    pub set: fn(&mut Engine, &mut Args) -> SourceResult<Styles>,
    pub vtable: fn(of: TypeId) -> Option<*const ()>,
    pub field_id: fn(name: &str) -> Option<u8>,
    pub field_name: fn(u8) -> Option<&'static str>,
    pub local_name: Option<fn(Lang, Option<Region>) -> &'static str>,
    pub scope: Lazy<Scope>,
    pub params: Lazy<Vec<ParamInfo>>,
}

impl From<&'static NativeElementData> for Element {
    fn from(data: &'static NativeElementData) -> Self {
        Self(Static(data))
    }
}

cast! {
    &'static NativeElementData,
    self => Element::from(self).into_value(),
}

/// Synthesize fields on an element. This happens before execution of any show
/// rule.
pub trait Synthesize {
    /// Prepare the element for show rule application.
    fn synthesize(&mut self, engine: &mut Engine, styles: StyleChain)
        -> SourceResult<()>;
}

/// The base recipe for an element.
pub trait Show {
    /// Execute the base recipe for this element.
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content>;
}

/// Post-process an element after it was realized.
pub trait Finalize {
    /// Finalize the fully realized form of the element. Use this for effects
    /// that should work even in the face of a user-defined show rule.
    fn finalize(&self, realized: Content, styles: StyleChain) -> Content;
}

/// How the element interacts with other elements.
pub trait Behave {
    /// The element's interaction behaviour.
    fn behaviour(&self) -> Behaviour;

    /// Whether this weak element is larger than a previous one and thus picked
    /// as the maximum when the levels are the same.
    #[allow(unused_variables)]
    fn larger(
        &self,
        prev: &(Cow<Content>, Behaviour, StyleChain),
        styles: StyleChain,
    ) -> bool {
        false
    }
}

/// How an element interacts with other elements in a stream.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Behaviour {
    /// A weak element which only survives when a supportive element is before
    /// and after it. Furthermore, per consecutive run of weak elements, only
    /// one survives: The one with the lowest weakness level (or the larger one
    /// if there is a tie).
    Weak(usize),
    /// An element that enables adjacent weak elements to exist. The default.
    Supportive,
    /// An element that destroys adjacent weak elements.
    Destructive,
    /// An element that does not interact at all with other elements, having the
    /// same effect as if it didn't exist, but has a visual representation.
    Ignorant,
    /// An element that does not have a visual representation.
    Invisible,
}

/// Guards content against being affected by the same show rule multiple times.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Guard {
    /// The nth recipe from the top of the chain.
    Nth(usize),
    /// The [base recipe](Show) for a kind of element.
    Base(Element),
}
