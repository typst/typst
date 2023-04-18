mod inner;

use std::any::TypeId;
use std::fmt::{self, Debug, Formatter, Write};
use std::iter::Sum;
use std::ops::{Add, AddAssign};

use comemo::Prehashed;
use ecow::{eco_format, EcoString};

use self::inner::{ContentInner, ContentTailItem};

use super::{
    element, Behave, Behaviour, ElemFunc, Element, Fold, Guard, Label, Locatable,
    Location, PlainText, Recipe, Selector, Style, Styles, Synthesize,
};
use crate::diag::{SourceResult, StrResult};
use crate::doc::Meta;
use crate::eval::{Cast, Str, Value, Vm};
use crate::syntax::Span;
use crate::util::pretty_array_like;

/// Composable representation of styled content.
#[derive(Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Content {
    func: ElemFunc,
    inner: ContentInner,
}

impl Content {
    /// Create an empty element.
    pub fn new(func: ElemFunc) -> Self {
        Self { func, inner: ContentInner::new() }
    }

    /// Create empty content.
    pub fn empty() -> Self {
        Self::new(SequenceElem::func())
    }

    /// Create a new sequence element from multiples elements.
    pub fn sequence<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Self>,
        I::IntoIter: ExactSizeIterator,
    {
        let mut iter = iter.into_iter();
        let Some(first) = iter.next() else { return Self::empty() };
        let Some(second) = iter.next() else { return first };

        Self {
            func: SequenceElem::func(),
            inner: ContentInner::with_iter(
                [first, second]
                    .into_iter()
                    .chain(iter)
                    .map(Prehashed::new)
                    .map(ContentTailItem::Child),
            ),
        }
    }

    /// The element function of the contained content.
    pub fn func(&self) -> ElemFunc {
        self.func
    }

    /// Whether the content is an empty sequence.
    pub fn is_empty(&self) -> bool {
        self.is::<SequenceElem>() && self.inner.is_childless()
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
        Some(self.inner.children())
    }

    /// Access the child and styles.
    pub fn to_styled(&self) -> Option<(&Content, &Styles)> {
        if !self.is::<StyledElem>() {
            return None;
        }
        let child = self.inner.children().next()?;
        let styles = self.inner.style()?;
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
        self.inner.span().copied().unwrap_or(Span::detached())
    }

    /// Attach a span to the content if it doesn't already have one.
    pub fn spanned(mut self, span: Span) -> Self {
        if self.span().is_detached() {
            self.inner.push_span(span);
        }

        self
    }

    /// Attach a field to the content.
    pub fn with_field(
        mut self,
        name: impl Into<EcoString>,
        value: impl Into<Value>,
    ) -> Self {
        self.push_field(name, value);

        self
    }

    pub fn push_field(&mut self, name: impl Into<EcoString>, value: impl Into<Value>) {
        self.inner.push_field(name.into(), value.into());
    }

    /// Access a field on the content.
    pub fn field(&self, name: &str) -> Option<Value> {
        if let (Some(iter), "children") = (self.to_sequence(), name) {
            Some(Value::Array(iter.cloned().map(Value::Content).collect()))
        } else if let (Some((child, _)), "child") = (self.to_styled(), "child") {
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
        self.inner.fields()
    }

    /// Try to access a field on the content as a specified type.
    pub fn cast_field<T: Cast>(&self, name: &str) -> Option<T> {
        match self.field(name) {
            Some(value) => value.cast().ok(),
            None => None,
        }
    }

    /// Expect a field on the content to exist as a specified type.
    #[track_caller]
    pub fn expect_field<T: Cast>(&self, name: &str) -> T {
        self.field(name).unwrap().cast().unwrap()
    }

    /// Whether the content has the specified field.
    pub fn has(&self, field: &str) -> bool {
        self.field(field).is_some()
    }

    /// Borrow the value of the given field.
    pub fn at(&self, field: &str) -> StrResult<Value> {
        self.field(field).ok_or_else(|| missing_field(field))
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
            self.inner.apply_style(style.into());
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
            self.inner.apply_styles(styles);
            self
        } else {
            let mut content = Content::new(StyledElem::func());
            content.inner.push_child(self);
            content.inner.apply_styles(styles);
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

    /// Repeat this content `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this content {} times", n))?;

        Ok(Self::sequence(vec![self.clone(); count]))
    }

    /// Disable a show rule recipe.
    pub fn guarded(mut self, guard: Guard) -> Self {
        self.inner.push_guard(guard);
        self
    }

    /// Check whether a show rule recipe is disabled.
    pub fn is_guarded(&self, guard: Guard) -> bool {
        self.inner.guards().any(|g| g == &guard)
    }

    /// Whether no show rule was executed for this content so far.
    pub fn is_pristine(&self) -> bool {
        self.inner.guards().next().is_none()
    }

    /// Whether this content has already been prepared.
    pub fn is_prepared(&self) -> bool {
        self.inner.is_prepared()
    }

    /// Mark this content as prepared.
    pub fn mark_prepared(&mut self) {
        self.inner.set_prepared(true);
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
        self.inner.location()
    }

    /// Attach a location to this content.
    pub fn set_location(&mut self, location: Location) {
        self.inner.push_location(location);
    }

    /// Queries the content tree for all elements that match the given selector.
    ///
    /// Elements produced in `show` rules will not be included in the results.
    pub fn query(&self, selector: Selector) -> Vec<&Content> {
        let mut results = Vec::new();
        self.traverse(&mut |element| {
            if selector.matches(element) {
                results.push(element);
            }
        });
        results
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

        for item in self.inner.slice() {
            match item {
                ContentTailItem::Child(child) => child.traverse(f),
                ContentTailItem::Field(_, value) => walk_value(value, f),
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
                lhs.inner.extend(rhs.inner);
                lhs
            }
            (true, false) => {
                lhs.inner.push_child(rhs);
                lhs
            }
            (false, true) => {
                rhs.inner.insert(0, ContentTailItem::Child(Prehashed::new(lhs)));
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
        Self::sequence(iter.collect::<Vec<_>>().into_iter())
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

impl Fold for Vec<Meta> {
    type Output = Self;

    fn fold(mut self, outer: Self::Output) -> Self::Output {
        self.extend(outer);
        self
    }
}

/// The missing key access error message.
#[cold]
#[track_caller]
fn missing_field(key: &str) -> EcoString {
    eco_format!("content does not contain field {:?}", Str::from(key))
}
