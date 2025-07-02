use std::any::TypeId;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::{LazyLock, OnceLock};

use ecow::EcoString;
use smallvec::SmallVec;
#[doc(inline)]
pub use typst_macros::elem;
use typst_utils::Static;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    cast, Args, CastInfo, Container, Content, FieldAccessError, Fold, Func, IntoValue,
    NativeScope, Packed, ParamInfo, Property, Reflect, Repr, Resolve, Scope, Selector,
    StyleChain, Styles, Value,
};
use crate::text::{Lang, LocalName, Region};

/// A document element.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Element(Static<NativeElementData>);

impl Element {
    /// Get the element for `T`.
    pub fn of<T: NativeElement>() -> Self {
        T::elem()
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
    pub fn vtable(self) -> fn(of: TypeId) -> Option<NonNull<()>> {
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

    /// Extract the field ID for the given field name.
    pub fn field_id(&self, name: &str) -> Option<u8> {
        if name == "label" {
            return Some(255);
        }
        (self.0.field_id)(name)
    }

    /// Extract the field name for the given field ID.
    pub fn field_name(&self, id: u8) -> Option<&'static str> {
        if id == 255 {
            return Some("label");
        }
        (self.0.field_name)(id)
    }

    /// Extract the value of the field for the given field ID and style chain.
    pub fn field_from_styles(
        &self,
        id: u8,
        styles: StyleChain,
    ) -> Result<Value, FieldAccessError> {
        (self.0.field_from_styles)(id, styles)
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

/// A Typst element that is defined by a native Rust type.
pub trait NativeElement:
    Debug + Clone + Hash + Construct + Set + Capable + Send + Sync + 'static
{
    const ELEMENT: TypedElementData<Self>;

    /// Get the element data for the native Rust element.
    fn data() -> &'static NativeElementData
    where
        Self: Sized;

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
}

impl<T: NativeElement> IntoValue for T {
    fn into_value(self) -> Value {
        Value::Content(self.pack())
    }
}

/// Used to cast an element to a trait object for a trait it implements.
///
/// # Safety
/// If the `vtable` function returns `Some(p)`, then `p` must be a valid pointer
/// to a vtable of `Packed<Self>` w.r.t to the trait `C` where `capability` is
/// `TypeId::of::<dyn C>()`.
pub unsafe trait Capable {
    /// Get the pointer to the vtable for the given capability / trait.
    fn vtable(capability: TypeId) -> Option<NonNull<()>>;
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

/// Defines a built-in show rule for an element.
pub trait Show {
    /// Execute the base recipe for this element.
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content>;
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

/// Type-erased metadata and routines for a native element.
#[derive(Debug)]
pub struct NativeElementData {
    /// The element's normal name (e.g. `align`), as exposed to Typst.
    pub name: &'static str,
    /// The element's title case name (e.g. `Align`).
    pub title: &'static str,
    /// The documentation for this element as a string.
    pub docs: &'static str,
    /// A list of alternate search terms for this element.
    pub keywords: &'static [&'static str],
    /// The constructor for this element (see [`Construct`]).
    pub construct: fn(&mut Engine, &mut Args) -> SourceResult<Content>,
    /// Executes this element's set rule (see [`Set`]).
    pub set: fn(&mut Engine, &mut Args) -> SourceResult<Styles>,
    /// Gets the vtable for one of this element's capabilities
    /// (see [`Capable`]).
    pub vtable: fn(capability: TypeId) -> Option<NonNull<()>>,
    /// Gets the numeric index of this field by its name.
    pub field_id: fn(name: &str) -> Option<u8>,
    /// Gets the name of a field by its numeric index.
    pub field_name: fn(u8) -> Option<&'static str>,
    /// Get the field with the given ID in the presence of styles.
    pub field_from_styles: fn(u8, StyleChain) -> Result<Value, FieldAccessError>,
    /// Gets the localized name for this element (see [`LocalName`][crate::text::LocalName]).
    pub local_name: Option<fn(Lang, Option<Region>) -> &'static str>,
    /// Associated definitions of the element.
    pub scope: LazyLock<Scope>,
    /// A list of parameter information for each field.
    pub params: LazyLock<Vec<ParamInfo>>,
}

impl NativeElementData {
    /// Creates type-erased element data for the given element.
    pub const fn new<E>() -> Self
    where
        E: NativeElement,
    {
        Self {
            name: E::ELEMENT.name,
            title: E::ELEMENT.title,
            docs: E::ELEMENT.docs,
            keywords: E::ELEMENT.keywords,
            local_name: E::ELEMENT.local_name,
            field_from_styles: |i, styles| match E::ELEMENT.get(i) {
                Some(field) => {
                    (field.get_from_styles)(styles).ok_or(FieldAccessError::Unknown)
                }
                None => Err(FieldAccessError::Unknown),
            },
            field_id: E::ELEMENT.field_id,
            field_name: |i| E::ELEMENT.get(i).map(|data| data.name),
            construct: <E as Construct>::construct,
            set: <E as Set>::set,
            vtable: <E as Capable>::vtable,
            scope: LazyLock::new(E::ELEMENT.scope),
            params: LazyLock::new(|| {
                E::ELEMENT
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
            }),
        }
    }
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

/// Type-aware metadata and routines for a native element.
pub struct TypedElementData<E: NativeElement> {
    pub name: &'static str,
    pub title: &'static str,
    pub docs: &'static str,
    pub keywords: &'static [&'static str],
    pub fields: &'static [TypedFieldData<E>],
    pub field_id: fn(name: &str) -> Option<u8>,
    pub repr: Option<fn(&E) -> EcoString>,
    pub eq: Option<fn(&E, &E) -> bool>,
    pub local_name: Option<fn(Lang, Option<Region>) -> &'static str>,
    pub scope: fn() -> Scope,
}

impl<E: NativeElement> TypedElementData<E> {
    /// Creates the data from its parts. This is called in the `#[elem]` macro.
    pub const fn new(
        name: &'static str,
        title: &'static str,
        docs: &'static str,
        fields: &'static [TypedFieldData<E>],
        field_id: fn(name: &str) -> Option<u8>,
    ) -> Self {
        Self {
            name,
            title,
            docs,
            keywords: &[],
            fields,
            field_id,
            repr: None,
            eq: None,
            local_name: None,
            scope: || Scope::new(),
        }
    }

    /// Attaches search keywords for the documentation.
    pub const fn with_keywords(self, keywords: &'static [&'static str]) -> Self {
        Self { keywords, ..self }
    }

    /// Takes a `Repr` impl into account.
    pub const fn with_repr(self) -> Self
    where
        E: Repr,
    {
        Self { repr: Some(E::repr), ..self }
    }

    /// Takes a `PartialEq` impl into account.
    pub const fn with_partial_eq(self) -> Self
    where
        E: PartialEq,
    {
        Self { eq: Some(E::eq), ..self }
    }

    /// Takes a `LocalName` impl into account.
    pub const fn with_local_name(self) -> Self
    where
        Packed<E>: LocalName,
    {
        Self {
            local_name: Some(<Packed<E> as LocalName>::local_name),
            ..self
        }
    }

    /// Takes a `NativeScope` impl into account.
    pub const fn with_scope(self) -> Self
    where
        E: NativeScope,
    {
        Self { scope: || E::scope(), ..self }
    }

    /// Retrieves the field with the given index.
    pub fn get(&self, i: u8) -> Option<&'static TypedFieldData<E>> {
        self.fields.get(usize::from(i))
    }
}

/// Metadata for a field and routines that is aware of concrete element, but
/// abstracts over what kind of field it is (required / variadic / synthesized /
/// settable / ghost).
pub struct TypedFieldData<E: NativeElement> {
    pub name: &'static str,
    pub docs: &'static str,
    pub positional: bool,
    pub variadic: bool,
    pub required: bool,
    pub settable: bool,
    pub synthesized: bool,
    pub input: fn() -> CastInfo,
    pub default: Option<fn() -> Value>,
    pub has: fn(content: &E) -> bool,
    pub get: fn(content: &E) -> Option<Value>,
    pub get_with_styles: fn(content: &E, StyleChain) -> Option<Value>,
    pub get_from_styles: fn(StyleChain) -> Option<Value>,
    pub materialize: fn(content: &mut E, styles: StyleChain),
    pub eq: fn(a: &E, b: &E) -> bool,
}

impl<E: NativeElement> TypedFieldData<E> {
    /// Creates type-erased metadata and routines for a `#[required]` field.
    pub const fn required<const I: u8>() -> Self
    where
        E: RequiredField<I>,
        E::Type: Reflect + IntoValue + PartialEq,
    {
        Self {
            name: E::FIELD.name,
            docs: E::FIELD.docs,
            positional: true,
            required: true,
            variadic: false,
            settable: false,
            synthesized: false,
            input: || <E::Type as Reflect>::input(),
            default: None,
            has: |_| true,
            get: |elem| Some((E::FIELD.get)(elem).clone().into_value()),
            get_with_styles: |elem, _| Some((E::FIELD.get)(elem).clone().into_value()),
            get_from_styles: |_| None,
            materialize: |_, _| {},
            eq: |a, b| (E::FIELD.get)(a) == (E::FIELD.get)(b),
        }
    }

    /// Creates type-erased metadata and routines for a `#[variadic]` field.
    pub const fn variadic<const I: u8>() -> Self
    where
        E: RequiredField<I>,
        E::Type: Container + IntoValue + PartialEq,
        <E::Type as Container>::Inner: Reflect,
    {
        Self {
            name: E::FIELD.name,
            docs: E::FIELD.docs,
            positional: true,
            required: true,
            variadic: true,
            settable: false,
            synthesized: false,
            input: || <<E::Type as Container>::Inner as Reflect>::input(),
            default: None,
            has: |_| true,
            get: |elem| Some((E::FIELD.get)(elem).clone().into_value()),
            get_with_styles: |elem, _| Some((E::FIELD.get)(elem).clone().into_value()),
            get_from_styles: |_| None,
            materialize: |_, _| {},
            eq: |a, b| (E::FIELD.get)(a) == (E::FIELD.get)(b),
        }
    }

    /// Creates type-erased metadata and routines for a `#[synthesized]` field.
    pub const fn synthesized<const I: u8>() -> Self
    where
        E: SynthesizedField<I>,
        E::Type: Reflect + IntoValue + PartialEq,
    {
        Self {
            name: E::FIELD.name,
            docs: E::FIELD.docs,
            positional: false,
            required: false,
            variadic: false,
            settable: false,
            synthesized: true,
            input: || <E::Type as Reflect>::input(),
            default: None,
            has: |elem| (E::FIELD.get)(elem).is_some(),
            get: |elem| (E::FIELD.get)(elem).clone().map(|v| v.into_value()),
            get_with_styles: |elem, _| {
                (E::FIELD.get)(elem).clone().map(|v| v.into_value())
            },
            get_from_styles: |_| None,
            materialize: |_, _| {},
            // Synthesized fields don't affect equality.
            eq: |_, _| true,
        }
    }

    /// Creates type-erased metadata and routines for a normal settable field.
    pub const fn settable<const I: u8>() -> Self
    where
        E: SettableField<I>,
        E::Type: Reflect + IntoValue + PartialEq,
    {
        Self {
            name: E::FIELD.property.name,
            docs: E::FIELD.property.docs,
            positional: E::FIELD.property.positional,
            required: false,
            variadic: false,
            settable: true,
            synthesized: false,
            input: || <E::Type as Reflect>::input(),
            default: Some(|| E::PROPERTY.default().into_value()),
            has: |elem| (E::FIELD.get)(elem).is_set(),
            get: |elem| (E::FIELD.get)(elem).as_option().clone().map(|v| v.into_value()),
            get_with_styles: |elem, styles| {
                Some((E::FIELD.get)(elem).get(styles).into_value())
            },
            get_from_styles: |styles| Some(styles.get::<E, I>(Field::new()).into_value()),
            materialize: |elem, styles| {
                if !(E::FIELD.get)(elem).is_set() {
                    (E::FIELD.get_mut)(elem).set(styles.get::<E, I>(Field::new()));
                }
            },
            eq: |a, b| (E::FIELD.get)(a).as_option() == (E::FIELD.get)(b).as_option(),
        }
    }

    /// Creates type-erased metadata and routines for a `#[ghost]` field.
    pub const fn ghost<const I: u8>() -> Self
    where
        E: SettableProperty<I>,
        E::Type: Reflect + IntoValue + PartialEq,
    {
        Self {
            name: E::PROPERTY.name,
            docs: E::PROPERTY.docs,
            positional: E::PROPERTY.positional,
            required: false,
            variadic: false,
            settable: true,
            synthesized: false,
            input: || <E::Type as Reflect>::input(),
            default: Some(|| E::PROPERTY.default().into_value()),
            has: |_| false,
            get: |_| None,
            get_with_styles: |_, styles| {
                Some(styles.get::<E, I>(Field::new()).into_value())
            },
            get_from_styles: |styles| Some(styles.get::<E, I>(Field::new()).into_value()),
            materialize: |_, _| {},
            eq: |_, _| true,
        }
    }

    /// Creates type-erased metadata and routines for an `#[external]` field.
    pub const fn external<const I: u8>() -> Self
    where
        E: ExternalField<I>,
        E::Type: Reflect + IntoValue,
    {
        Self {
            name: E::FIELD.name,
            docs: E::FIELD.docs,
            positional: false,
            required: false,
            variadic: false,
            settable: false,
            synthesized: false,
            input: || <E::Type as Reflect>::input(),
            default: Some(|| (E::FIELD.default)().into_value()),
            has: |_| false,
            get: |_| None,
            get_with_styles: |_, _| None,
            get_from_styles: |_| None,
            materialize: |_, _| {},
            eq: |_, _| true,
        }
    }
}

/// A field that is present on every instance of the element.
pub trait RequiredField<const I: u8>: NativeElement {
    type Type: Clone;

    const FIELD: RequiredFieldData<Self, I>;
}

/// Metadata and routines for a [`RequiredField`].
pub struct RequiredFieldData<E: RequiredField<I>, const I: u8> {
    pub name: &'static str,
    pub docs: &'static str,
    pub get: fn(&E) -> &E::Type,
    pub get_mut: fn(&mut E) -> &mut E::Type,
}

impl<E: RequiredField<I>, const I: u8> RequiredFieldData<E, I> {
    /// Creates the data from its parts. This is called in the `#[elem]` macro.
    pub const fn new(
        name: &'static str,
        docs: &'static str,
        get: fn(&E) -> &E::Type,
        get_mut: fn(&mut E) -> &mut E::Type,
    ) -> Self {
        Self { name, docs, get, get_mut }
    }
}

/// A field that is initially unset, but may be set through a [`Synthesize`]
/// implementation.
pub trait SynthesizedField<const I: u8>: NativeElement {
    type Type: Clone;

    const FIELD: SynthesizedFieldData<Self, I>;
}

/// Metadata and routines for a [`SynthesizedField`].
pub struct SynthesizedFieldData<E: SynthesizedField<I>, const I: u8> {
    pub name: &'static str,
    pub docs: &'static str,
    pub get: fn(&E) -> &Option<E::Type>,
    pub get_mut: fn(&mut E) -> &mut Option<E::Type>,
}

impl<E: SynthesizedField<I>, const I: u8> SynthesizedFieldData<E, I> {
    /// Creates the data from its parts. This is called in the `#[elem]` macro.
    pub const fn new(
        name: &'static str,
        docs: &'static str,
        get: fn(&E) -> &Option<E::Type>,
        get_mut: fn(&mut E) -> &mut Option<E::Type>,
    ) -> Self {
        Self { name, docs, get, get_mut }
    }
}

/// A field that has a default value and can be configured via a set rule, but
/// can also present on elements and be present in the constructor.
pub trait SettableField<const I: u8>: NativeElement {
    type Type: Clone;

    const FIELD: SettableFieldData<Self, I>;
}

/// Metadata and routines for a [`SettableField`].
pub struct SettableFieldData<E: SettableField<I>, const I: u8> {
    pub get: fn(&E) -> &Settable<E, I>,
    pub get_mut: fn(&mut E) -> &mut Settable<E, I>,
    pub property: SettablePropertyData<E, I>,
}

impl<E: SettableField<I>, const I: u8> SettableFieldData<E, I> {
    /// Creates the data from its parts. This is called in the `#[elem]` macro.
    pub const fn new(
        name: &'static str,
        docs: &'static str,
        positional: bool,
        get: fn(&E) -> &Settable<E, I>,
        get_mut: fn(&mut E) -> &mut Settable<E, I>,
        default: fn() -> E::Type,
        slot: fn() -> &'static OnceLock<E::Type>,
    ) -> Self {
        Self {
            get,
            get_mut,
            property: SettablePropertyData::new(name, docs, positional, default, slot),
        }
    }

    /// Ensures that the property is folded on every access. See the
    /// documentation of the [`Fold`] trait for more details.
    pub const fn with_fold(mut self) -> Self
    where
        E::Type: Fold,
    {
        self.property.fold = Some(E::Type::fold);
        self
    }
}

/// A field that has a default value and can be configured via a set rule, but
/// is never present on elements.
///
/// This is provided for all `SettableField` impls through a blanket impl. In
/// the case of `#[ghost]` fields, which only live in the style chain and not in
/// elements, it is also implemented manually.
pub trait SettableProperty<const I: u8>: NativeElement {
    type Type: Clone;

    const PROPERTY: SettablePropertyData<Self, I>;
}

impl<T, const I: u8> SettableProperty<I> for T
where
    T: SettableField<I>,
{
    type Type = <Self as SettableField<I>>::Type;

    const PROPERTY: SettablePropertyData<Self, I> =
        <Self as SettableField<I>>::FIELD.property;
}

/// Metadata and routines for a [`SettableProperty`].
pub struct SettablePropertyData<E: SettableProperty<I>, const I: u8> {
    pub name: &'static str,
    pub docs: &'static str,
    pub positional: bool,
    pub default: fn() -> E::Type,
    pub slot: fn() -> &'static OnceLock<E::Type>,
    #[allow(clippy::type_complexity)]
    pub fold: Option<fn(E::Type, E::Type) -> E::Type>,
}

impl<E: SettableProperty<I>, const I: u8> SettablePropertyData<E, I> {
    /// Creates the data from its parts. This is called in the `#[elem]` macro.
    pub const fn new(
        name: &'static str,
        docs: &'static str,
        positional: bool,
        default: fn() -> E::Type,
        slot: fn() -> &'static OnceLock<E::Type>,
    ) -> Self {
        Self { name, docs, positional, default, slot, fold: None }
    }

    /// Ensures that the property is folded on every access. See the
    /// documentation of the [`Fold`] trait for more details.
    pub const fn with_fold(self) -> Self
    where
        E::Type: Fold,
    {
        Self { fold: Some(E::Type::fold), ..self }
    }

    /// Produces an instance of the property's default value.
    pub fn default(&self) -> E::Type
    where
        E::Type: Clone,
    {
        // Avoid recreating an expensive instance over and over, but also
        // avoid unnecessary lazy initialization for cheap types.
        if std::mem::needs_drop::<E::Type>() {
            self.default_ref().clone()
        } else {
            (self.default)()
        }
    }

    /// Produces a static reference to this property's default value.
    pub fn default_ref(&self) -> &'static E::Type {
        (self.slot)().get_or_init(self.default)
    }
}

/// A settable property that can be accessed by reference (because it is not
/// folded).
pub trait RefableProperty<const I: u8>: SettableProperty<I> {}

/// A settable field of an element.
///
/// The field can be in two states: Unset or present.
#[derive(Copy, Clone, Hash)]
pub struct Settable<E: NativeElement, const I: u8>(Option<E::Type>)
where
    E: SettableProperty<I>;

impl<E: NativeElement, const I: u8> Settable<E, I>
where
    E: SettableProperty<I>,
{
    /// Creates a new unset instance.
    pub fn new() -> Self {
        Self(None)
    }

    /// Sets the instance to a value.
    pub fn set(&mut self, value: E::Type) {
        self.0 = Some(value);
    }

    /// Clears the value from the instance.
    pub fn unset(&mut self) {
        self.0 = None;
    }

    /// Views the type as an [`Option`] which is `Some` if the type is set
    /// and `None` if it is unset.
    pub fn as_option(&self) -> &Option<E::Type> {
        &self.0
    }

    /// Views the type as a mutable [`Option`].
    pub fn as_option_mut(&mut self) -> &mut Option<E::Type> {
        &mut self.0
    }

    /// Whether the field is set.
    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// Retrieves the value given styles. The styles are used if the value is
    /// unset or if it needs folding.
    pub fn get<'a>(&'a self, styles: StyleChain<'a>) -> E::Type
    where
        E::Type: Clone,
    {
        styles.get_with::<E, I>(Field::new(), self.0.as_ref())
    }

    /// Retrieves a reference to the value given styles. Not possible if the
    /// value needs folding.
    pub fn get_ref<'a>(&'a self, styles: StyleChain<'a>) -> &'a E::Type
    where
        E: RefableProperty<I>,
    {
        styles.get_ref_with::<E, I>(Field::new(), self.0.as_ref())
    }

    /// Retrieves the value and then immediately [resolves](Resolve) it.
    pub fn resolve<'a>(&'a self, styles: StyleChain<'a>) -> <E::Type as Resolve>::Output
    where
        E::Type: Resolve + Clone,
    {
        self.get(styles).resolve(styles)
    }
}

impl<E: NativeElement, const I: u8> Debug for Settable<E, I>
where
    E: SettableProperty<I>,
    E::Type: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<E: NativeElement, const I: u8> Default for Settable<E, I>
where
    E: SettableProperty<I>,
{
    fn default() -> Self {
        Self(None)
    }
}

impl<E: NativeElement, const I: u8> From<Option<E::Type>> for Settable<E, I>
where
    E: SettableProperty<I>,
{
    fn from(value: Option<E::Type>) -> Self {
        Self(value)
    }
}

/// An accessor for the `I`-th field of the element `E`. Values of this type are
/// generated for each field of an element can be used to interact with this
/// field programmatically, for example to access the style chain, as in
/// `styles.get(TextElem::size)`.
#[derive(Copy, Clone)]
pub struct Field<E: NativeElement, const I: u8>(pub PhantomData<E>);

impl<E: NativeElement, const I: u8> Field<E, I> {
    /// Creates a new zero-sized accessor.
    pub const fn new() -> Self {
        Self(PhantomData)
    }

    /// The index of the projected field.
    pub const fn index(self) -> u8 {
        I
    }

    /// Creates a dynamic property instance for this field.
    ///
    /// Prefer [`Content::set`] or [`Styles::set`] when working with existing
    /// content or style value.
    pub fn set(self, value: E::Type) -> Property
    where
        E: SettableProperty<I>,
        E::Type: Debug + Clone + Hash + Send + Sync + 'static,
    {
        Property::new(self, value)
    }
}

impl<E: NativeElement, const I: u8> Default for Field<E, I> {
    fn default() -> Self {
        Self::new()
    }
}

/// A field that is not actually there. It's only visible in the docs.
pub trait ExternalField<const I: u8>: NativeElement {
    type Type;

    const FIELD: ExternalFieldData<Self, I>;
}

/// Metadata for an [`ExternalField`].
pub struct ExternalFieldData<E: ExternalField<I>, const I: u8> {
    pub name: &'static str,
    pub docs: &'static str,
    pub default: fn() -> E::Type,
}

impl<E: ExternalField<I>, const I: u8> ExternalFieldData<E, I> {
    /// Creates the data from its parts. This is called in the `#[elem]` macro.
    pub const fn new(
        name: &'static str,
        docs: &'static str,
        default: fn() -> E::Type,
    ) -> Self {
        Self { name, docs, default }
    }
}
