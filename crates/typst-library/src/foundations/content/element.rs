use std::any::TypeId;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::sync::OnceLock;

use ecow::EcoString;
use smallvec::SmallVec;
use typst_utils::Static;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    Args, Content, ContentVtable, FieldAccessError, Func, ParamInfo, Repr, Scope,
    Selector, StyleChain, Styles, Value, cast,
};
use crate::text::{Lang, Region};

/// A document element.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Element(Static<ContentVtable>);

impl Element {
    /// Get the element for `T`.
    pub const fn of<T: NativeElement>() -> Self {
        T::ELEM
    }

    /// Get the element for `T`.
    pub const fn from_vtable(vtable: &'static ContentVtable) -> Self {
        Self(Static(vtable))
    }

    /// The element's normal name (e.g. `enum`).
    pub fn name(self) -> &'static str {
        self.vtable().name
    }

    /// The element's title case name, for use in documentation
    /// (e.g. `Numbered List`).
    pub fn title(&self) -> &'static str {
        self.vtable().title
    }

    /// Documentation for the element (as Markdown).
    pub fn docs(&self) -> &'static str {
        self.vtable().docs
    }

    /// Search keywords for the element.
    pub fn keywords(&self) -> &'static [&'static str] {
        self.vtable().keywords
    }

    /// Construct an instance of this element.
    pub fn construct(
        self,
        engine: &mut Engine,
        args: &mut Args,
    ) -> SourceResult<Content> {
        (self.vtable().construct)(engine, args)
    }

    /// Execute the set rule for the element and return the resulting style map.
    pub fn set(self, engine: &mut Engine, mut args: Args) -> SourceResult<Styles> {
        let styles = (self.vtable().set)(engine, &mut args)?;
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
        (self.vtable().capability)(type_id).is_some()
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
        (self.vtable().store)().scope.get_or_init(|| (self.vtable().scope)())
    }

    /// Details about the element's fields.
    pub fn params(&self) -> &'static [ParamInfo] {
        (self.vtable().store)().params.get_or_init(|| {
            self.vtable()
                .fields
                .iter()
                .filter(|field| !field.synthesized)
                .map(|field| ParamInfo {
                    name: field.name,
                    docs: field.docs,
                    input: (field.input)(),
                    default: field.default,
                    positional: field.positional,
                    named: !field.positional,
                    variadic: field.variadic,
                    required: field.required,
                    settable: field.settable,
                })
                .collect()
        })
    }

    /// Extract the field ID for the given field name.
    pub fn field_id(&self, name: &str) -> Option<u8> {
        if name == "label" {
            return Some(255);
        }
        (self.vtable().field_id)(name)
    }

    /// Extract the field name for the given field ID.
    pub fn field_name(&self, id: u8) -> Option<&'static str> {
        if id == 255 {
            return Some("label");
        }
        self.vtable().field(id).map(|data| data.name)
    }

    /// Extract the value of the field for the given field ID and style chain.
    pub fn field_from_styles(
        &self,
        id: u8,
        styles: StyleChain,
    ) -> Result<Value, FieldAccessError> {
        self.vtable()
            .field(id)
            .and_then(|field| (field.get_from_styles)(styles))
            .ok_or(FieldAccessError::Unknown)
    }

    /// The element's local name, if any.
    pub fn local_name(&self, lang: Lang, region: Option<Region>) -> Option<&'static str> {
        self.vtable().local_name.map(|f| f(lang, region))
    }

    /// Retrieves the element's vtable for dynamic dispatch.
    pub(super) fn vtable(&self) -> &'static ContentVtable {
        (self.0).0
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
    v: Func => v.to_element().ok_or("expected element")?,
}

/// Lazily initialized data for an element.
#[derive(Default)]
pub struct LazyElementStore {
    pub scope: OnceLock<Scope>,
    pub params: OnceLock<Vec<ParamInfo>>,
}

impl LazyElementStore {
    /// Create an empty store.
    pub const fn new() -> Self {
        Self { scope: OnceLock::new(), params: OnceLock::new() }
    }
}

/// A Typst element that is defined by a native Rust type.
///
/// # Safety
/// `ELEM` must hold the correct `Element` for `Self`.
pub unsafe trait NativeElement:
    Debug + Clone + Hash + Construct + Set + Send + Sync + 'static
{
    /// The associated element.
    const ELEM: Element;

    /// Pack the element into type-erased content.
    fn pack(self) -> Content {
        Content::new(self)
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

/// Synthesize fields on an element. This happens before execution of any show
/// rule.
pub trait Synthesize {
    /// Prepare the element for show rule application.
    fn synthesize(&mut self, engine: &mut Engine, styles: StyleChain)
    -> SourceResult<()>;
}

/// Defines built-in show set rules for an element.
///
/// This is a bit more powerful than a user-defined show-set because it can
/// access the element's fields.
pub trait ShowSet {
    /// Finalize the fully realized form of the element. Use this for effects
    /// that should work even in the face of a user-defined show rule.
    fn show_set(&self, styles: StyleChain) -> Styles;
}

/// Tries to extract the plain-text representation of the element.
pub trait PlainText {
    /// Write this element's plain text into the given buffer.
    fn plain_text(&self, text: &mut EcoString);
}
