use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use comemo::Prehashed;
use ecow::{eco_format, EcoString, EcoVec};
use serde::{Serialize, Serializer};
use typst_macros::selem;

use super::{
    Behave, Behaviour, Element, ElementData, Guard, Label, Locatable, Location,
    NativeElement, Recipe, Selector, Style, Styles, Synthesize,
};
use crate::diag::{SourceResult, StrResult};
use crate::doc::Meta;
use crate::eval::{func, scope, ty, Dict, FromValue, IntoValue, Repr, Str, Value, Vm};
use crate::syntax::Span;

#[ty(scope)]
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct Content(pub Arc<dyn Element>);

impl Default for Content {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: Element> From<T> for Content {
    fn from(value: T) -> Self {
        Self::static_(value)
    }
}

impl From<Arc<dyn Element>> for Content {
    fn from(value: Arc<dyn Element>) -> Self {
        Self(value)
    }
}

impl Content {
    #[inline]
    pub fn static_<E: Element>(elem: E) -> Self {
        Self(Arc::new(elem))
    }

    #[inline]
    pub fn empty() -> Self {
        Self::static_(SequenceElem::default())
    }

    pub fn temp(of: ElementData) -> Self {
        of.empty()
    }

    #[inline]
    pub fn get(&self, name: &str) -> StrResult<Value> {
        self.field(name).ok_or_else(|| missing_field(name))
    }

    #[inline]
    pub fn field(&self, name: &str) -> Option<Value> {
        self.0.field(name)
    }

    #[inline]
    pub fn span(&self) -> Span {
        self.0.span()
    }

    #[inline]
    pub fn label(&self) -> Option<&Label> {
       self.0.label()
    }

    pub fn spanned(mut self, span: Span) -> Self {
        swap_with_mut(&mut self.0);
        Arc::get_mut(&mut self.0).unwrap().set_span(span);
        self
    }

    pub fn labelled(mut self, label: Label) -> Self {
        swap_with_mut(&mut self.0);
        Arc::get_mut(&mut self.0).unwrap().set_label(label);
        self
    }

    pub fn set_location(&mut self, location: Location) {
        swap_with_mut(&mut self.0);
        Arc::get_mut(&mut self.0).unwrap().set_location(location);
    }

    /// Create a new sequence element from multiples elements.
    pub fn sequence(iter: impl IntoIterator<Item = Self>) -> Self {
        let mut iter = iter.into_iter();
        let Some(first) = iter.next() else { return Self::empty() };
        let Some(second) = iter.next() else { return first };
        SequenceElem::new(
            std::iter::once(Prehashed::new(first))
            .chain(std::iter::once(Prehashed::new(second)))
            .chain(iter.map(Prehashed::new))
            .collect()
        ).into()
    }

    /// Access the children if this is a sequence.
    pub fn to_sequence(&self) -> Option<impl Iterator<Item = &Content>> {
        let Some(sequence) = SequenceElem::unpack(self) else {
            return None;
        };

        Some(sequence.children.iter().map(std::ops::Deref::deref))
    }

    pub fn elem(&self) -> ElementData {
        self.0.data()
    }

    /// Whether the contained element is of type `T`.
    pub fn is<T: NativeElement>(&self) -> bool {
        self.elem() == T::elem()
    }

    /// Cast to `T` if the contained element is of type `T`.
    pub fn to<T: NativeElement>(&self) -> Option<&T> {
        T::unpack(self)
    }

    /// Cast to `T` if the contained element is of type `T`.
    pub fn to_mut<T: NativeElement>(&mut self) -> Option<&mut T> {
        T::unpack_mut(self)
    }

    /// Whether the contained element has the given capability.
    pub fn can<C>(&self) -> bool
    where
        C: ?Sized + 'static,
    {
        self.elem().can::<C>()
    }

    /// Whether the contained element has the given capability where the
    /// capability is given by a `TypeId`.
    pub fn can_type_id(&self, type_id: TypeId) -> bool {
        self.elem().can_type_id(type_id)
    }

    /// Cast to a trait object if the contained element has the given
    /// capability.
    pub fn with<C>(&self) -> Option<&C>
    where
        C: ?Sized + 'static,
    {
        let vtable = self.elem().vtable()(TypeId::of::<C>())?;
        let data = Arc::as_ptr(&self.0) as *const ();
        Some(unsafe { &*crate::util::fat::from_raw_parts(data, vtable) })
    }

    pub fn with_mut<C>(&mut self) -> Option<&mut C>
    where
        C: ?Sized + 'static,
    {
        let vtable = self.elem().vtable()(TypeId::of::<C>())?;
        let data = Arc::as_ptr(&mut self.0) as *const () as *mut ();
        Some(unsafe { &mut *crate::util::fat::from_raw_parts_mut(data, vtable) })
    }

    pub fn is_sequence(&self) -> bool {
        self.is::<SequenceElem>()
    }

    /// Whether the content is an empty sequence.
    pub fn is_empty(&self) -> bool {
        let Some(sequence) = SequenceElem::unpack(self) else {
            return false;
        };

        sequence.children.is_empty()
    }

    /// Also auto expands sequence of sequences into flat sequence
    pub fn sequence_recursive_for_each(&self, f: &mut impl FnMut(&Self)) {
        if let Some(childs) = self.to_sequence() {
            childs.for_each(|c| c.sequence_recursive_for_each(f));
        } else {
            f(self);
        }
    }

    /// Access the child and styles.
    pub fn to_styled(&self) -> Option<(&Content, &Styles)> {
        let styled = StyledElem::unpack(self)?;

        let child = &styled.child;
        let styles = &styled.styles;
        Some((child, styles))
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
        Self::sequence(std::iter::repeat_with(|| self.clone()).take(count))
    }

    /// Style this content with a style entry.
    pub fn styled(mut self, style: impl Into<Style>) -> Self {
        if let Some(style_elem) = StyledElem::unpack_mut(&mut self) {
            style_elem.styles.apply_one(style.into());
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

        if let Some(style_elem) = StyledElem::unpack_mut(&mut self) {
            style_elem.styles.apply(styles.into());
            self
        } else {
            StyledElem::new(Prehashed::new(self), styles).into()
        }
    }

    /// Whether the content needs to be realized specially.
    pub fn needs_preparation(&self) -> bool {
        (self.can::<dyn Locatable>()
            || self.can::<dyn Synthesize>()
            || self.label().is_some())
            && !self.is_prepared()
    }

    /// Queries the content tree for all elements that match the given selector.
    ///
    /// Elements produced in `show` rules will not be included in the results.
    #[tracing::instrument(skip_all)]
    pub fn query(&self, selector: Selector) -> Vec<Content> {
        let mut results = Vec::new();
        self.traverse(&mut |element| {
            if selector.matches(&element) {
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
    pub fn query_first(&self, selector: Selector) -> Option<Content> {
        let mut result = None;
        self.traverse(&mut |element| {
            if result.is_none() && selector.matches(&element) {
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

    pub fn fields_ref(&self) -> EcoVec<(EcoString, Value)> {
        self.0
            .fields()
            .into_iter()
            .map(|(key, value)| (key.0, value))
            .collect()
    }

    /// Traverse this content.
    fn traverse<F>(&self, f: &mut F)
    where
        F: FnMut(Content),
    {
        f(self.clone());

        self.0.fields().into_iter().for_each(|(_, value)| walk_value(value, f));

        /// Walks a given value to find any content that matches the selector.
        fn walk_value<F>(value: Value, f: &mut F)
        where
            F: FnMut(Content),
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

    /// Disable a show rule recipe.
    pub fn guarded(mut self, guard: Guard) -> Self {
        swap_with_mut(&mut self.0);
        Arc::get_mut(&mut self.0).unwrap().push_guard(guard);
        self.0.into()
    }

    /// Check whether a show rule recipe is disabled.
    pub fn is_guarded(&self, guard: Guard) -> bool {
        self.0.is_guarded(guard)
    }

    /// Whether no show rule was executed for this content so far.
    pub fn is_pristine(&self) -> bool {
        self.0.is_pristine()
    }

    /// Expect a field on the content to exist as a specified type.
    #[track_caller]
    pub fn expect_field<T: FromValue>(&self, name: &str) -> T {
        self.field(name).unwrap().cast().unwrap()
    }

    /// Whether this content has already been prepared.
    pub fn is_prepared(&self) -> bool {
        self.0.is_prepared()
    }

    /// Mark this content as prepared.
    pub fn mark_prepared(&mut self) {
        swap_with_mut(&mut self.0);
        Arc::get_mut(&mut self.0).unwrap().mark_prepared();
    }

    /// Attach a field to the content.
    pub fn with_field(mut self, name: &str, value: impl IntoValue) -> Self {
        swap_with_mut(&mut self.0);
        Arc::get_mut(&mut self.0)
            .unwrap()
            .set_field(name, value.into_value())
            .unwrap();
        self
    }
}

impl std::hash::Hash for Content {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.data().hash(state);
        self.0.dyn_hash(state)
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(left), Some(right)) = (self.to_styled(), other.to_styled()) {
            left == right
        }  else {
            self.0.dyn_eq(&other.0 as &dyn Any)
        }
    }
}

impl Repr for Content {
    fn repr(&self) -> EcoString {
        self.0.repr()
    }
}


#[scope]
impl Content {
    /// The content's element function. This function can be used to create the element
    /// contained in this content. It can be used in set and show rules for the
    /// element. Can be compared with global functions to check whether you have
    /// a specific
    /// kind of element.
    #[func]
    pub fn func(&self) -> ElementData {
        self.elem()
    }

    /// Whether the content has the specified field.
    #[func]
    pub fn has(
        &self,
        /// The field to look for.
        field: Str,
    ) -> bool {
        self.field(&field).is_some()
    }

    /// Access the specified field on the content. Returns the default value if
    /// the field does not exist or fails with an error if no default value was
    /// specified.
    #[func]
    pub fn at(
        &self,
        /// The field to access.
        field: Str,
        /// A default value to return if the field does not exist.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        self.field(&field)
            .or(default)
            .ok_or_else(|| missing_field_no_default(&field))
    }

    /// Returns the fields of this content.
    ///
    /// ```example
    /// #rect(
    ///   width: 10cm,
    ///   height: 10cm,
    /// ).fields()
    /// ```
    #[func]
    pub fn fields(&self) -> Dict {
        static CHILD: EcoString = EcoString::inline("child");
        static CHILDREN: EcoString = EcoString::inline("children");

        let option = if let Some(iter) = self.to_sequence() {
            Some((
                CHILDREN.clone(),
                Value::Array(iter.cloned().map(Value::Content).collect()),
            ))
        } else if let Some((child, _)) = self.to_styled() {
            Some((CHILD.clone(), Value::Content(child.clone())))
        } else {
            None
        };

        self.fields_ref()
            .into_iter()
            .chain(option)
            .map(|(key, value)| (Str::from(key), value))
            .collect()
    }

    /// The location of the content. This is only available on content returned
    /// by [query]($query) or provided by a
    /// [show rule]($reference/styling/#show-rules), for other content it will
    /// be `{none}`. The resulting location can be used with
    /// [counters]($counter), [state]($state) and [queries]($query).
    #[func]
    pub fn location(&self) -> Option<Location> {
       self.0.location()
    }
}

impl<'a> Add<&'a Content> for Content {
    type Output = Self;

    fn add(self, rhs: &'a Content) -> Self::Output {
        let mut lhs = self;
        match (lhs.to_mut::<SequenceElem>(), rhs.to::<SequenceElem>()) {
            (Some(seq_lhs), Some(rhs)) => {
                seq_lhs.children.extend(rhs.children.iter().cloned());
                lhs
            }
            (Some(seq_lhs), None) => {
                seq_lhs.children.push(Prehashed::new(rhs.clone()));
                lhs
            }
            (None, Some(_)) => {
                let mut rhs = rhs.clone();
                rhs.to_mut::<SequenceElem>().unwrap().children.insert(0, Prehashed::new(lhs));
                rhs
            }
            (None, None) => Self::sequence([lhs, rhs.clone()]),
        }
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, mut rhs: Self) -> Self::Output {
        let mut lhs = self;
        match (lhs.to_mut::<SequenceElem>(), rhs.to_mut::<SequenceElem>()) {
            (Some(seq_lhs), Some(rhs)) => {
                seq_lhs.children.extend(rhs.children.iter().cloned());
                lhs
            }
            (Some(seq_lhs), None) => {
                seq_lhs.children.push(Prehashed::new(rhs));
                lhs
            }
            (None, Some(rhs_seq)) => {
                rhs_seq.children.insert(0, Prehashed::new(lhs));
                rhs
            }
            (None, None) => Self::sequence([lhs, rhs]),
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
        serializer.collect_map(iter::once((EcoString::inline("func"), self.func().name().into_value()))
            .chain(self.fields().into_iter().map(|(key, value)| (key.0, value))))
    }
}

/// Defines the `ElemFunc` for sequences.
#[selem]
struct SequenceElem {
    #[required]
    #[children]
    #[empty(Vec::with_capacity(0))]
    children: Vec<Prehashed<Content>>,
}

/// Defines the `ElemFunc` for styled elements.
#[selem]
struct StyledElem {
    #[required]
    child: Prehashed<Content>,
    #[required]
    styles: Styles,
}

/// Hosts metadata and ensures metadata is produced even for empty elements.
#[selem(Behave)]
pub struct MetaElem {
    /// Metadata that should be attached to all elements affected by this style
    /// property.
    #[fold]
    pub data: Vec<Meta>,
}

impl Behave for MetaElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Invisible
    }
}

/// Tries to extract the plain-text representation of the element.
pub trait PlainText {
    /// Write this element's plain text into the given buffer.
    fn plain_text(&self, text: &mut EcoString);
}

/// The missing field access error message.
#[cold]
fn missing_field(field: &str) -> EcoString {
    eco_format!("content does not contain field {}", field.repr())
}

/// The missing field access error message when no default value was given.
#[cold]
fn missing_field_no_default(field: &str) -> EcoString {
    eco_format!(
        "content does not contain field {} and \
         no default value was specified",
        field.repr()
    )
}

#[doc(hidden)]
#[allow(invalid_value)]
pub fn swap_with_mut(val: &mut Arc<dyn Element>) {
    match Arc::get_mut(val) {
        Some(_) => {}
        None => *val = val.dyn_clone(),
    };
}
