use std::fmt::{self, Debug};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::OnceLock;

use ecow::{eco_format, EcoString};

use crate::foundations::{
    Container, Content, FieldVtable, Fold, FoldFn, IntoValue, NativeElement, Packed,
    Property, Reflect, Repr, Resolve, StyleChain,
};

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
    /// Prefer [`Content::set`] or
    /// [`Styles::set`](crate::foundations::Styles::set) when working with
    /// existing content or style value.
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

/// A field that is present on every instance of the element.
pub trait RequiredField<const I: u8>: NativeElement {
    type Type: Clone;

    const FIELD: RequiredFieldData<Self, I>;
}

/// Metadata and routines for a [`RequiredField`].
pub struct RequiredFieldData<E: RequiredField<I>, const I: u8> {
    name: &'static str,
    docs: &'static str,
    get: fn(&E) -> &E::Type,
}

impl<E: RequiredField<I>, const I: u8> RequiredFieldData<E, I> {
    /// Creates the data from its parts. This is called in the `#[elem]` macro.
    pub const fn new(
        name: &'static str,
        docs: &'static str,
        get: fn(&E) -> &E::Type,
    ) -> Self {
        Self { name, docs, get }
    }

    /// Creates the vtable for a `#[required]` field.
    pub const fn vtable() -> FieldVtable<Packed<E>>
    where
        E: RequiredField<I>,
        E::Type: Reflect + IntoValue + PartialEq,
    {
        FieldVtable {
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

    /// Creates the vtable for a `#[variadic]` field.
    pub const fn vtable_variadic() -> FieldVtable<Packed<E>>
    where
        E: RequiredField<I>,
        E::Type: Container + IntoValue + PartialEq,
        <E::Type as Container>::Inner: Reflect,
    {
        FieldVtable {
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
}

/// A field that is initially unset, but may be set through a
/// [`Synthesize`](crate::foundations::Synthesize) implementation.
pub trait SynthesizedField<const I: u8>: NativeElement {
    type Type: Clone;

    const FIELD: SynthesizedFieldData<Self, I>;
}

/// Metadata and routines for a [`SynthesizedField`].
pub struct SynthesizedFieldData<E: SynthesizedField<I>, const I: u8> {
    name: &'static str,
    docs: &'static str,
    get: fn(&E) -> &Option<E::Type>,
}

impl<E: SynthesizedField<I>, const I: u8> SynthesizedFieldData<E, I> {
    /// Creates the data from its parts. This is called in the `#[elem]` macro.
    pub const fn new(
        name: &'static str,
        docs: &'static str,
        get: fn(&E) -> &Option<E::Type>,
    ) -> Self {
        Self { name, docs, get }
    }

    /// Creates type-erased metadata and routines for a `#[synthesized]` field.
    pub const fn vtable() -> FieldVtable<Packed<E>>
    where
        E: SynthesizedField<I>,
        E::Type: Reflect + IntoValue + PartialEq,
    {
        FieldVtable {
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
}

/// A field that is not actually there. It's only visible in the docs.
pub trait ExternalField<const I: u8>: NativeElement {
    type Type;

    const FIELD: ExternalFieldData<Self, I>;
}

/// Metadata for an [`ExternalField`].
pub struct ExternalFieldData<E: ExternalField<I>, const I: u8> {
    name: &'static str,
    docs: &'static str,
    default: fn() -> E::Type,
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

    /// Creates type-erased metadata and routines for an `#[external]` field.
    pub const fn vtable() -> FieldVtable<Packed<E>>
    where
        E: ExternalField<I>,
        E::Type: Reflect + IntoValue,
    {
        FieldVtable {
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

/// A field that has a default value and can be configured via a set rule, but
/// can also present on elements and be present in the constructor.
pub trait SettableField<const I: u8>: NativeElement {
    type Type: Clone;

    const FIELD: SettableFieldData<Self, I>;
}

/// Metadata and routines for a [`SettableField`].
pub struct SettableFieldData<E: SettableField<I>, const I: u8> {
    get: fn(&E) -> &Settable<E, I>,
    get_mut: fn(&mut E) -> &mut Settable<E, I>,
    property: SettablePropertyData<E, I>,
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

    /// Creates type-erased metadata and routines for a normal settable field.
    pub const fn vtable() -> FieldVtable<Packed<E>>
    where
        E: SettableField<I>,
        E::Type: Reflect + IntoValue + PartialEq,
    {
        FieldVtable {
            name: E::FIELD.property.name,
            docs: E::FIELD.property.docs,
            positional: E::FIELD.property.positional,
            required: false,
            variadic: false,
            settable: true,
            synthesized: false,
            input: || <E::Type as Reflect>::input(),
            default: Some(|| E::default().into_value()),
            has: |elem| (E::FIELD.get)(elem).is_set(),
            get: |elem| (E::FIELD.get)(elem).as_option().clone().map(|v| v.into_value()),
            get_with_styles: |elem, styles| {
                Some((E::FIELD.get)(elem).get_cloned(styles).into_value())
            },
            get_from_styles: |styles| {
                Some(styles.get_cloned::<E, I>(Field::new()).into_value())
            },
            materialize: |elem, styles| {
                if !(E::FIELD.get)(elem).is_set() {
                    (E::FIELD.get_mut)(elem).set(styles.get_cloned::<E, I>(Field::new()));
                }
            },
            eq: |a, b| (E::FIELD.get)(a).as_option() == (E::FIELD.get)(b).as_option(),
        }
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

    const FIELD: SettablePropertyData<Self, I>;
    const FOLD: Option<FoldFn<Self::Type>> = Self::FIELD.fold;

    /// Produces an instance of the property's default value.
    fn default() -> Self::Type {
        // Avoid recreating an expensive instance over and over, but also
        // avoid unnecessary lazy initialization for cheap types.
        if std::mem::needs_drop::<Self::Type>() {
            Self::default_ref().clone()
        } else {
            (Self::FIELD.default)()
        }
    }

    /// Produces a static reference to this property's default value.
    fn default_ref() -> &'static Self::Type {
        (Self::FIELD.slot)().get_or_init(Self::FIELD.default)
    }
}

impl<T, const I: u8> SettableProperty<I> for T
where
    T: SettableField<I>,
{
    type Type = <Self as SettableField<I>>::Type;

    const FIELD: SettablePropertyData<Self, I> =
        <Self as SettableField<I>>::FIELD.property;
}

/// Metadata and routines for a [`SettableProperty`].
pub struct SettablePropertyData<E: SettableProperty<I>, const I: u8> {
    name: &'static str,
    docs: &'static str,
    positional: bool,
    default: fn() -> E::Type,
    slot: fn() -> &'static OnceLock<E::Type>,
    fold: Option<FoldFn<E::Type>>,
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

    /// Creates type-erased metadata and routines for a `#[ghost]` field.
    pub const fn vtable() -> FieldVtable<Packed<E>>
    where
        E: SettableProperty<I>,
        E::Type: Reflect + IntoValue + PartialEq,
    {
        FieldVtable {
            name: E::FIELD.name,
            docs: E::FIELD.docs,
            positional: E::FIELD.positional,
            required: false,
            variadic: false,
            settable: true,
            synthesized: false,
            input: || <E::Type as Reflect>::input(),
            default: Some(|| E::default().into_value()),
            has: |_| false,
            get: |_| None,
            get_with_styles: |_, styles| {
                Some(styles.get_cloned::<E, I>(Field::new()).into_value())
            },
            get_from_styles: |styles| {
                Some(styles.get_cloned::<E, I>(Field::new()).into_value())
            },
            materialize: |_, _| {},
            eq: |_, _| true,
        }
    }
}

/// A settable property that can be accessed by reference (because it is not
/// folded).
pub trait RefableProperty<const I: u8>: SettableProperty<I> {}

/// A settable field of an element.
///
/// The field can be in two states: Unset or present.
///
/// See [`StyleChain`] for more details about the available accessor methods.
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
    /// unset.
    pub fn get<'a>(&'a self, styles: StyleChain<'a>) -> E::Type
    where
        E::Type: Copy,
    {
        self.get_cloned(styles)
    }

    /// Retrieves and clones the value given styles. The styles are used if the
    /// value is unset or if it needs folding.
    pub fn get_cloned<'a>(&'a self, styles: StyleChain<'a>) -> E::Type {
        if let Some(fold) = E::FOLD {
            let mut res = styles.get_cloned::<E, I>(Field::new());
            if let Some(value) = &self.0 {
                res = fold(value.clone(), res);
            }
            res
        } else if let Some(value) = &self.0 {
            value.clone()
        } else {
            styles.get_cloned::<E, I>(Field::new())
        }
    }

    /// Retrieves a reference to the value given styles. The styles are used if
    /// the value is unset.
    pub fn get_ref<'a>(&'a self, styles: StyleChain<'a>) -> &'a E::Type
    where
        E: RefableProperty<I>,
    {
        if let Some(value) = &self.0 {
            value
        } else {
            styles.get_ref::<E, I>(Field::new())
        }
    }

    /// Retrieves the value and then immediately [resolves](Resolve) it.
    pub fn resolve<'a>(&'a self, styles: StyleChain<'a>) -> <E::Type as Resolve>::Output
    where
        E::Type: Resolve,
    {
        self.get_cloned(styles).resolve(styles)
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

/// An error arising when trying to access a field of content.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum FieldAccessError {
    Unknown,
    Unset,
}

impl FieldAccessError {
    /// Formats the error message given the content and the field name.
    #[cold]
    pub fn message(self, content: &Content, field: &str) -> EcoString {
        let elem_name = content.elem().name();
        match self {
            FieldAccessError::Unknown => {
                eco_format!("{elem_name} does not have field {}", field.repr())
            }
            FieldAccessError::Unset => {
                eco_format!(
                    "field {} in {elem_name} is not known at this point",
                    field.repr()
                )
            }
        }
    }

    /// Formats the error message for an `at` calls without a default value.
    #[cold]
    pub fn message_no_default(self, content: &Content, field: &str) -> EcoString {
        let mut msg = self.message(content, field);
        msg.push_str(" and no default was specified");
        msg
    }
}
