use std::any::TypeId;
use std::fmt::{self, Debug, Formatter, Write};
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign};

use comemo::Prehashed;
use ecow::{eco_format, EcoString, EcoVec};
use serde::{Serialize, Serializer};

use super::{
    element, Behave, Behaviour, ElemFunc, Element, Guard, Label, Locatable, Location,
    Recipe, Selector, Style, Styles, Synthesize,
};
use crate::diag::{SourceResult, StrResult};
use crate::doc::Meta;
use crate::eval::{Dict, FromValue, IntoValue, Str, Value, Vm};
use crate::syntax::Span;
use crate::util::pretty_array_like;

/// Composable representation of styled content.
#[derive(Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Content {
    func: ElemFunc,
    attrs: EcoVec<Attr>,
}

/// Attributes that can be attached to content.
#[derive(Debug, Clone, PartialEq, Hash)]
enum Attr {
    Span(Span),
    Field(EcoString),
    Value(Prehashed<Value>),
    Child(Prehashed<Content>),
    Styles(Styles),
    Prepared,
    Guard(Guard),
    Location(Location),
}

impl Content {
    /// Create an empty element.
    pub fn new(func: ElemFunc) -> Self {
        Self { func, attrs: EcoVec::new() }
    }

    /// Create empty content.
    pub fn empty() -> Self {
        Self::new(SequenceElem::func())
    }

    /// Create a new sequence element from multiples elements.
    pub fn sequence(iter: impl IntoIterator<Item = Self>) -> Self {
        let mut iter = iter.into_iter();
        let Some(first) = iter.next() else { return Self::empty() };
        let Some(second) = iter.next() else { return first };
        let mut content = Content::empty();
        content.attrs.push(Attr::Child(Prehashed::new(first)));
        content.attrs.push(Attr::Child(Prehashed::new(second)));
        content
            .attrs
            .extend(iter.map(|child| Attr::Child(Prehashed::new(child))));
        content
    }

    /// The element function of the contained content.
    pub fn func(&self) -> ElemFunc {
        self.func
    }

    /// Whether the content is an empty sequence.
    pub fn is_empty(&self) -> bool {
        self.is::<SequenceElem>() && self.attrs.is_empty()
    }

    /// Whether the contained element is of type `T`.
    pub fn is<T: Element>(&self) -> bool {
        self.func == T::func()
    }

    /// Cast to `T` if the contained element is of type `T`.
    pub fn to<T: Element>(&self) -> Option<&T> {
        T::unpack(self)
    }

    /// Access the children if this is a sequence.
    pub fn to_sequence(&self) -> Option<impl Iterator<Item = &Self>> {
        if !self.is::<SequenceElem>() {
            return None;
        }
        Some(self.attrs.iter().filter_map(Attr::child))
    }

    /// Access the child and styles.
    pub fn to_styled(&self) -> Option<(&Content, &Styles)> {
        if !self.is::<StyledElem>() {
            return None;
        }
        let child = self.attrs.iter().find_map(Attr::child)?;
        let styles = self.attrs.iter().find_map(Attr::styles)?;
        Some((child, styles))
    }

    /// Whether the contained element has the given capability.
    pub fn can<C>(&self) -> bool
    where
        C: ?Sized + 'static,
    {
        (self.func.0.vtable)(TypeId::of::<C>()).is_some()
    }

    /// Whether the contained element has the given capability.
    /// Where the capability is given by a `TypeId`.
    pub fn can_type_id(&self, type_id: TypeId) -> bool {
        (self.func.0.vtable)(type_id).is_some()
    }

    /// Cast to a trait object if the contained element has the given
    /// capability.
    pub fn with<C>(&self) -> Option<&C>
    where
        C: ?Sized + 'static,
    {
        let vtable = (self.func.0.vtable)(TypeId::of::<C>())?;
        let data = self as *const Self as *const ();
        Some(unsafe { &*crate::util::fat::from_raw_parts(data, vtable) })
    }

    /// Cast to a mutable trait object if the contained element has the given
    /// capability.
    pub fn with_mut<C>(&mut self) -> Option<&mut C>
    where
        C: ?Sized + 'static,
    {
        let vtable = (self.func.0.vtable)(TypeId::of::<C>())?;
        let data = self as *mut Self as *mut ();
        Some(unsafe { &mut *crate::util::fat::from_raw_parts_mut(data, vtable) })
    }

    /// The content's span.
    pub fn span(&self) -> Span {
        self.attrs.iter().find_map(Attr::span).unwrap_or(Span::detached())
    }

    /// Attach a span to the content if it doesn't already have one.
    pub fn spanned(mut self, span: Span) -> Self {
        if self.span().is_detached() {
            self.attrs.push(Attr::Span(span));
        }
        self
    }

    /// Attach a field to the content.
    pub fn with_field(
        mut self,
        name: impl Into<EcoString>,
        value: impl IntoValue,
    ) -> Self {
        self.push_field(name, value);
        self
    }

    /// Attach a field to the content.
    pub fn push_field(&mut self, name: impl Into<EcoString>, value: impl IntoValue) {
        let name = name.into();
        if let Some(i) = self.attrs.iter().position(|attr| match attr {
            Attr::Field(field) => *field == name,
            _ => false,
        }) {
            self.attrs.make_mut()[i + 1] =
                Attr::Value(Prehashed::new(value.into_value()));
        } else {
            self.attrs.push(Attr::Field(name));
            self.attrs.push(Attr::Value(Prehashed::new(value.into_value())));
        }
    }

    /// Access a field on the content.
    pub fn field(&self, name: &str) -> Option<Value> {
        if let (Some(iter), "children") = (self.to_sequence(), name) {
            Some(Value::Array(iter.cloned().map(Value::Content).collect()))
        } else if let (Some((child, _)), "child") = (self.to_styled(), name) {
            Some(Value::Content(child.clone()))
        } else {
            self.field_ref(name).cloned()
        }
    }

    /// Access a field on the content by reference.
    ///
    /// Does not include synthesized fields for sequence and styled elements.
    pub fn field_ref(&self, name: &str) -> Option<&Value> {
        self.fields_ref()
            .find(|&(field, _)| field == name)
            .map(|(_, value)| value)
    }

    /// Iter over all fields on the content.
    ///
    /// Does not include synthesized fields for sequence and styled elements.
    pub fn fields(&self) -> impl Iterator<Item = (&EcoString, Value)> {
        static CHILD: EcoString = EcoString::inline("child");
        static CHILDREN: EcoString = EcoString::inline("children");

        let option = if let Some(iter) = self.to_sequence() {
            Some((&CHILDREN, Value::Array(iter.cloned().map(Value::Content).collect())))
        } else if let Some((child, _)) = self.to_styled() {
            Some((&CHILD, Value::Content(child.clone())))
        } else {
            None
        };

        self.fields_ref()
            .map(|(name, value)| (name, value.clone()))
            .chain(option)
    }

    /// Iter over all fields on the content.
    ///
    /// Does not include synthesized fields for sequence and styled elements.
    pub fn fields_ref(&self) -> impl Iterator<Item = (&EcoString, &Value)> {
        let mut iter = self.attrs.iter();
        std::iter::from_fn(move || {
            let field = iter.find_map(Attr::field)?;
            let value = iter.next()?.value()?;
            Some((field, value))
        })
    }

    /// Try to access a field on the content as a specified type.
    pub fn cast_field<T: FromValue>(&self, name: &str) -> Option<T> {
        match self.field(name) {
            Some(value) => value.cast().ok(),
            None => None,
        }
    }

    /// Expect a field on the content to exist as a specified type.
    #[track_caller]
    pub fn expect_field<T: FromValue>(&self, name: &str) -> T {
        self.field(name).unwrap().cast().unwrap()
    }

    /// Whether the content has the specified field.
    pub fn has(&self, field: &str) -> bool {
        self.field(field).is_some()
    }

    /// Borrow the value of the given field.
    pub fn at(&self, field: &str, default: Option<Value>) -> StrResult<Value> {
        self.field(field)
            .or(default)
            .ok_or_else(|| missing_field_no_default(field))
    }

    /// Return the fields of the content as a dict.
    pub fn dict(&self) -> Dict {
        self.fields()
            .map(|(key, value)| (key.to_owned().into(), value))
            .collect()
    }

    /// The content's label.
    pub fn label(&self) -> Option<&Label> {
        match self.field_ref("label")? {
            Value::Label(label) => Some(label),
            _ => None,
        }
    }

    /// Attach a label to the content.
    pub fn labelled(self, label: Label) -> Self {
        self.with_field("label", label)
    }

    /// Style this content with a style entry.
    pub fn styled(mut self, style: impl Into<Style>) -> Self {
        if self.is::<StyledElem>() {
            let prev =
                self.attrs.make_mut().iter_mut().find_map(Attr::styles_mut).unwrap();
            prev.apply_one(style.into());
            self
        } else {
            self.styled_with_map(style.into().into())
        }
    }

    /// Style this content with a full style map.
    pub fn styled_with_map(mut self, styles: Styles) -> Self {
        if styles.is_empty() {
            return self;
        }

        if self.is::<StyledElem>() {
            let prev =
                self.attrs.make_mut().iter_mut().find_map(Attr::styles_mut).unwrap();
            prev.apply(styles);
            self
        } else {
            let mut content = Content::new(StyledElem::func());
            content.attrs.push(Attr::Child(Prehashed::new(self)));
            content.attrs.push(Attr::Styles(styles));
            content
        }
    }

    /// Style this content with a recipe, eagerly applying it if possible.
    pub fn styled_with_recipe(self, vm: &mut Vm, recipe: Recipe) -> SourceResult<Self> {
        if recipe.selector.is_none() {
            recipe.apply_vm(vm, self)
        } else {
            Ok(self.styled(recipe))
        }
    }

    /// Repeat this content `count` times.
    pub fn repeat(&self, count: usize) -> Self {
        Self::sequence(vec![self.clone(); count])
    }

    /// Disable a show rule recipe.
    pub fn guarded(mut self, guard: Guard) -> Self {
        self.attrs.push(Attr::Guard(guard));
        self
    }

    /// Check whether a show rule recipe is disabled.
    pub fn is_guarded(&self, guard: Guard) -> bool {
        self.attrs.contains(&Attr::Guard(guard))
    }

    /// Whether no show rule was executed for this content so far.
    pub fn is_pristine(&self) -> bool {
        !self.attrs.iter().any(|modifier| matches!(modifier, Attr::Guard(_)))
    }

    /// Whether this content has already been prepared.
    pub fn is_prepared(&self) -> bool {
        self.attrs.contains(&Attr::Prepared)
    }

    /// Mark this content as prepared.
    pub fn mark_prepared(&mut self) {
        self.attrs.push(Attr::Prepared);
    }

    /// Whether the content needs to be realized specially.
    pub fn needs_preparation(&self) -> bool {
        (self.can::<dyn Locatable>()
            || self.can::<dyn Synthesize>()
            || self.label().is_some())
            && !self.is_prepared()
    }

    /// This content's location in the document flow.
    pub fn location(&self) -> Option<Location> {
        self.attrs.iter().find_map(|modifier| match modifier {
            Attr::Location(location) => Some(*location),
            _ => None,
        })
    }

    /// Attach a location to this content.
    pub fn set_location(&mut self, location: Location) {
        self.attrs.push(Attr::Location(location));
    }

    /// Queries the content tree for all elements that match the given selector.
    ///
    /// Elements produced in `show` rules will not be included in the results.
    #[tracing::instrument(skip_all)]
    pub fn query(&self, selector: Selector) -> Vec<&Content> {
        let mut results = Vec::new();
        self.traverse(&mut |element| {
            if selector.matches(element) {
                results.push(element);
            }
        });
        results
    }

    /// Queries the content tree for the first element that match the given
    /// selector.
    ///
    /// Elements produced in `show` rules will not be included in the results.
    #[tracing::instrument(skip_all)]
    pub fn query_first(&self, selector: Selector) -> Option<&Content> {
        let mut result = None;
        self.traverse(&mut |element| {
            if result.is_none() && selector.matches(element) {
                result = Some(element);
            }
        });
        result
    }

    /// Extracts the plain text of this content.
    pub fn plain_text(&self) -> EcoString {
        let mut text = EcoString::new();
        self.traverse(&mut |element| {
            if let Some(textable) = element.with::<dyn PlainText>() {
                textable.plain_text(&mut text);
            }
        });
        text
    }

    /// Traverse this content.
    fn traverse<'a, F>(&'a self, f: &mut F)
    where
        F: FnMut(&'a Content),
    {
        f(self);

        for attr in &self.attrs {
            match attr {
                Attr::Child(child) => child.traverse(f),
                Attr::Value(value) => walk_value(value, f),
                _ => {}
            }
        }

        /// Walks a given value to find any content that matches the selector.
        fn walk_value<'a, F>(value: &'a Value, f: &mut F)
        where
            F: FnMut(&'a Content),
        {
            match value {
                Value::Content(content) => content.traverse(f),
                Value::Array(array) => {
                    for value in array {
                        walk_value(value, f);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Debug for Content {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let name = self.func.name();
        if let Some(text) = item!(text_str)(self) {
            f.write_char('[')?;
            f.write_str(&text)?;
            f.write_char(']')?;
            return Ok(());
        } else if name == "space" {
            return f.write_str("[ ]");
        }

        let mut pieces: Vec<_> = self
            .fields()
            .map(|(name, value)| eco_format!("{name}: {value:?}"))
            .collect();

        if self.is::<StyledElem>() {
            pieces.push(EcoString::from(".."));
        }

        f.write_str(name)?;
        f.write_str(&pretty_array_like(&pieces, false))
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(left), Some(right)) = (self.to_sequence(), other.to_sequence()) {
            left.eq(right)
        } else if let (Some(left), Some(right)) = (self.to_styled(), other.to_styled()) {
            left == right
        } else {
            self.func == other.func && self.fields_ref().eq(other.fields_ref())
        }
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, mut rhs: Self) -> Self::Output {
        let mut lhs = self;
        match (lhs.is::<SequenceElem>(), rhs.is::<SequenceElem>()) {
            (true, true) => {
                lhs.attrs.extend(rhs.attrs);
                lhs
            }
            (true, false) => {
                lhs.attrs.push(Attr::Child(Prehashed::new(rhs)));
                lhs
            }
            (false, true) => {
                rhs.attrs.insert(0, Attr::Child(Prehashed::new(lhs)));
                rhs
            }
            (false, false) => Self::sequence([lhs, rhs]),
        }
    }
}

impl AddAssign for Content {
    fn add_assign(&mut self, rhs: Self) {
        *self = std::mem::take(self) + rhs;
    }
}

impl Sum for Content {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::sequence(iter)
    }
}

impl Serialize for Content {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(
            iter::once((&"func".into(), self.func().name().into_value()))
                .chain(self.fields()),
        )
    }
}

impl Attr {
    fn child(&self) -> Option<&Content> {
        match self {
            Self::Child(child) => Some(child),
            _ => None,
        }
    }

    fn styles(&self) -> Option<&Styles> {
        match self {
            Self::Styles(styles) => Some(styles),
            _ => None,
        }
    }

    fn styles_mut(&mut self) -> Option<&mut Styles> {
        match self {
            Self::Styles(styles) => Some(styles),
            _ => None,
        }
    }

    fn field(&self) -> Option<&EcoString> {
        match self {
            Self::Field(field) => Some(field),
            _ => None,
        }
    }

    fn value(&self) -> Option<&Value> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }

    fn span(&self) -> Option<Span> {
        match self {
            Self::Span(span) => Some(*span),
            _ => None,
        }
    }
}

/// Display: Sequence
/// Category: special
#[element]
struct SequenceElem {}

/// Display: Sequence
/// Category: special
#[element]
struct StyledElem {}

/// Hosts metadata and ensures metadata is produced even for empty elements.
///
/// Display: Meta
/// Category: special
#[element(Behave)]
pub struct MetaElem {
    /// Metadata that should be attached to all elements affected by this style
    /// property.
    #[fold]
    pub data: Vec<Meta>,
}

impl Behave for MetaElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}

/// Tries to extract the plain-text representation of the element.
pub trait PlainText {
    /// Write this element's plain text into the given buffer.
    fn plain_text(&self, text: &mut EcoString);
}

/// The missing field access error message when no default value was given.
#[cold]
fn missing_field_no_default(field: &str) -> EcoString {
    eco_format!(
        "content does not contain field {:?} and \
         no default value was specified",
        Str::from(field)
    )
}
