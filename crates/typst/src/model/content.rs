use std::any::TypeId;
use std::fmt::Debug;
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use comemo::Prehashed;
use ecow::{eco_format, EcoString};
use serde::{Serialize, Serializer};
use smallvec::SmallVec;
use typst_macros::elem;

use super::{
    Behave, Behaviour, Element, Guard, Label, Location, NativeElement, Recipe, Selector,
    Style, Styles,
};
use crate::diag::{SourceResult, StrResult};
use crate::doc::Meta;
use crate::eval::{func, scope, ty, Dict, FromValue, IntoValue, Repr, Str, Value, Vm};
use crate::syntax::Span;
use crate::util::pretty_array_like;

/// A piece of document content.
///
/// This type is at the heart of Typst. All markup you write and most
/// [functions]($function) you call produce content values. You can create a
/// content value by enclosing markup in square brackets. This is also how you
/// pass content to functions.
///
/// # Example
/// ```example
/// Type of *Hello!* is
/// #type([*Hello!*])
/// ```
///
/// Content can be added with the `+` operator,
/// [joined together]($scripting/#blocks) and multiplied with integers. Wherever
/// content is expected, you can also pass a [string]($str) or `{none}`.
///
/// # Representation
/// Content consists of elements with fields. When constructing an element with
/// its _element function,_ you provide these fields as arguments and when you
/// have a content value, you can access its fields with [field access
/// syntax]($scripting/#field-access).
///
/// Some fields are required: These must be provided when constructing an
/// element and as a consequence, they are always available through field access
/// on content of that type. Required fields are marked as such in the
/// documentation.
///
/// Most fields are optional: Like required fields, they can be passed to the
/// element function to configure them for a single element. However, these can
/// also be configured with [set rules]($styling/#set-rules) to apply them to
/// all elements within a scope. Optional fields are only available with field
/// access syntax when they are were explicitly passed to the element function,
/// not when they result from a set rule.
///
/// Each element has a default appearance. However, you can also completely
/// customize its appearance with a [show rule]($styling/#show-rules). The show
/// rule is passed the element. It can access the element's field and produce
/// arbitrary content from it.
///
/// In the web app, you can hover over a content variable to see exactly which
/// elements the content is composed of and what fields they have.
/// Alternatively, you can inspect the output of the [`repr`]($repr) function.
#[ty(scope)]
#[derive(Debug, Clone)]
pub struct Content(Arc<dyn NativeElement>);

impl Content {
    /// Creates a new content from an element.
    #[inline]
    pub fn new<E: NativeElement>(elem: E) -> Self {
        Self(Arc::new(elem))
    }

    /// Creates a new content from a already shared element.
    #[inline]
    pub fn from_shared<E: NativeElement>(elem: Arc<E>) -> Self {
        Self(elem)
    }

    /// Creates a new empty sequence content.
    #[inline]
    pub fn empty() -> Self {
        Self::new(SequenceElem::default())
    }

    /// Get the element data of this content.
    pub fn elem(&self) -> Element {
        self.0.dyn_data()
    }

    /// Get the span of the content.
    #[inline]
    pub fn span(&self) -> Span {
        self.0.span()
    }

    /// Set the span of the content.
    pub fn spanned(mut self, span: Span) -> Self {
        make_mut(&mut self.0).set_span(span);
        self
    }

    /// Get the label of the content.
    #[inline]
    pub fn label(&self) -> Option<Label> {
        self.0.label()
    }

    /// Set the label of the content.
    pub fn labelled(mut self, label: Label) -> Self {
        make_mut(&mut self.0).set_label(label);
        self
    }

    /// Set the location of the content.
    pub fn set_location(&mut self, location: Location) {
        make_mut(&mut self.0).set_location(location);
    }

    /// Disable a show rule recipe.
    pub fn guarded(mut self, guard: Guard) -> Self {
        make_mut(&mut self.0).push_guard(guard);
        self.0.into()
    }

    /// Whether the content needs to be realized specially.
    pub fn needs_preparation(&self) -> bool {
        self.0.needs_preparation()
    }

    /// Check whether a show rule recipe is disabled.
    pub fn is_guarded(&self, guard: Guard) -> bool {
        self.0.is_guarded(guard)
    }

    /// Whether no show rule was executed for this content so far.
    pub fn is_pristine(&self) -> bool {
        self.0.is_pristine()
    }

    /// Whether this content has already been prepared.
    pub fn is_prepared(&self) -> bool {
        self.0.is_prepared()
    }

    /// Mark this content as prepared.
    pub fn mark_prepared(&mut self) {
        make_mut(&mut self.0).mark_prepared();
    }

    /// Get a field by name.
    ///
    /// This can potentially allocate a new string.
    /// If you have access to the field IDs of the elemnt, use [`Self::get`]
    /// instead.
    #[inline]
    pub fn get_by_name(&self, name: &str) -> StrResult<Value> {
        let id = self.elem().field_id(name).ok_or_else(|| missing_field(name))?;
        self.get(id)
    }

    /// Get a field by ID, returning a missing field error
    /// if it does not exist.
    ///
    /// This is the preferred way to access fields.
    /// Only use this if you have set the field IDs yourself or are using
    /// the field IDs generated by the `#[elem]` macro.
    #[inline]
    pub fn get(&self, id: u8) -> StrResult<Value> {
        self.field(id)
            .ok_or_else(|| missing_field(self.elem().field_name(id).unwrap()))
    }

    /// Get a field by ID.
    ///
    /// This is the preferred way to access fields.
    /// Only use this if you have set the field IDs yourself or are using
    /// the field IDs generated by the `#[elem]` macro.
    #[inline]
    pub fn field(&self, id: u8) -> Option<Value> {
        self.0.field(id)
    }

    /// Set a field to the content.
    pub fn with_field(mut self, id: u8, value: impl IntoValue) -> Self {
        make_mut(&mut self.0).set_field(id, value.into_value()).unwrap();
        self
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
                .collect(),
        )
        .into()
    }

    /// Access the children if this is a sequence.
    pub fn to_sequence(&self) -> Option<impl Iterator<Item = &Prehashed<Content>>> {
        let Some(sequence) = self.to::<SequenceElem>() else {
            return None;
        };

        Some(sequence.children.iter())
    }

    /// Whether the contained element is of type `T`.
    pub fn is<T: NativeElement>(&self) -> bool {
        self.elem() == T::elem()
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
        let data = self.as_ptr() as *const ();
        Some(unsafe { &*crate::util::fat::from_raw_parts(data, vtable) })
    }

    /// Cast to a mutable trait object if the contained element has the given
    /// capability.
    pub fn with_mut<C>(&mut self) -> Option<&mut C>
    where
        C: ?Sized + 'static,
    {
        // Safety: we ensure the element is not shared.
        let vtable = self.elem().vtable()(TypeId::of::<C>())?;
        let data = self.as_mut_ptr() as *mut ();
        Some(unsafe { &mut *crate::util::fat::from_raw_parts_mut(data, vtable) })
    }

    /// Whether the content is a sequence.
    pub fn is_sequence(&self) -> bool {
        self.is::<SequenceElem>()
    }

    /// Whether the content is an empty sequence.
    pub fn is_empty(&self) -> bool {
        let Some(sequence) = self.to::<SequenceElem>() else {
            return false;
        };

        sequence.children.is_empty()
    }

    /// Also auto expands sequence of sequences into flat sequence
    pub fn sequence_recursive_for_each(&self, f: &mut impl FnMut(&Self)) {
        if let Some(children) = self.to_sequence() {
            children.for_each(|c| c.sequence_recursive_for_each(f));
        } else {
            f(self);
        }
    }

    /// Access the child and styles.
    pub fn to_styled(&self) -> Option<(&Content, &Styles)> {
        let styled = self.to::<StyledElem>()?;

        let child = styled.child();
        let styles = styled.styles();
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
        if let Some(style_elem) = self.to_mut::<StyledElem>() {
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

        if let Some(style_elem) = self.to_mut::<StyledElem>() {
            style_elem.styles.apply(styles);
            self
        } else {
            StyledElem::new(Prehashed::new(self), styles).into()
        }
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

    /// Traverse this content.
    fn traverse<F>(&self, f: &mut F)
    where
        F: FnMut(Content),
    {
        f(self.clone());

        self.0
            .fields()
            .into_iter()
            .for_each(|(_, value)| walk_value(value, f));

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

    /// Expect a field on the content to exist as a specified type.
    #[track_caller]
    pub fn expect_field<T: FromValue>(&self, id: u8) -> T {
        self.field(id).unwrap().cast().unwrap()
    }

    /// Expect a field on the content to exist as a specified type.
    #[track_caller]
    pub fn expect_field_by_name<T: FromValue>(&self, name: &str) -> T {
        self.get_by_name(name).unwrap().cast().unwrap()
    }

    /// Returns a pointer to the content.
    #[inline]
    pub fn as_ptr(&self) -> *const dyn NativeElement {
        Arc::as_ptr(&self.0)
    }

    /// Returns a mutable pointer to the content.
    ///
    /// Ensures that the content is not shared.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut dyn NativeElement {
        make_mut(&mut self.0) as *mut dyn NativeElement
    }

    /// Unwraps this content into its inner element.
    #[inline]
    pub fn into_inner(self) -> Arc<dyn NativeElement> {
        self.0
    }

    /// Downcast the element into an owned value.
    #[inline]
    pub fn unpack<T: NativeElement>(self) -> Option<Arc<T>> {
        // Early check for performance.
        if T::elem() != self.elem() {
            return None;
        }

        let t = self.0.to_any();
        Arc::downcast(t).ok()
    }

    /// Downcasts the element to the specified type.
    #[inline]
    pub fn to<T: NativeElement>(&self) -> Option<&T> {
        // Early check for performance.
        if T::elem() != self.elem() {
            return None;
        }

        self.0.as_any().downcast_ref()
    }

    /// Downcasts mutably the element to the specified type.
    #[inline]
    pub fn to_mut<T: NativeElement>(&mut self) -> Option<&mut T> {
        // Early check for performance.
        if T::elem() != self.elem() {
            return None;
        }

        make_mut(&mut self.0).as_any_mut().downcast_mut()
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
    pub fn func(&self) -> Element {
        self.elem()
    }

    /// Whether the content has the specified field.
    #[func]
    pub fn has(
        &self,
        /// The field to look for.
        field: Str,
    ) -> bool {
        let Some(id) = self.elem().field_id(&field) else {
            return false;
        };

        self.field(id).is_some()
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
        let Some(id) = self.elem().field_id(&field) else {
            return default.ok_or_else(|| missing_field_no_default(&field));
        };

        self.field(id)
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
        self.0.fields()
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

impl Default for Content {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: NativeElement> From<T> for Content {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl From<Arc<dyn NativeElement>> for Content {
    fn from(value: Arc<dyn NativeElement>) -> Self {
        Self(value)
    }
}

impl std::hash::Hash for Content {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.dyn_hash(state);
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        // Additional short circuit for different elements.
        self.elem() == other.elem() && self.0.dyn_eq(other)
    }
}

impl Repr for Content {
    fn repr(&self) -> EcoString {
        self.0.repr()
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
                rhs.to_mut::<SequenceElem>()
                    .unwrap()
                    .children
                    .insert(0, Prehashed::new(lhs));
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
        serializer.collect_map(
            iter::once((
                Str::from(EcoString::inline("func")),
                self.func().name().into_value(),
            ))
            .chain(self.fields()),
        )
    }
}

/// Defines the `ElemFunc` for sequences.
#[elem(Repr, PartialEq)]
struct SequenceElem {
    #[required]
    children: Vec<Prehashed<Content>>,
}

impl Default for SequenceElem {
    fn default() -> Self {
        Self {
            span: Span::detached(),
            location: Default::default(),
            label: Default::default(),
            prepared: Default::default(),
            guards: Default::default(),
            children: Default::default(),
        }
    }
}

impl PartialEq for SequenceElem {
    fn eq(&self, other: &Self) -> bool {
        self.children
            .iter()
            .map(|c| &**c)
            .eq(other.children.iter().map(|c| &**c))
    }
}

impl Repr for SequenceElem {
    fn repr(&self) -> EcoString {
        if self.children.is_empty() {
            EcoString::inline("[]")
        } else {
            eco_format!(
                "[{}]",
                pretty_array_like(
                    &self.children.iter().map(|c| c.0.repr()).collect::<Vec<_>>(),
                    false
                )
            )
        }
    }
}

/// Defines the `ElemFunc` for styled elements.
#[elem(Repr, PartialEq)]
struct StyledElem {
    #[required]
    child: Prehashed<Content>,
    #[required]
    styles: Styles,
}

impl PartialEq for StyledElem {
    fn eq(&self, other: &Self) -> bool {
        *self.child == *other.child
    }
}

impl Repr for StyledElem {
    fn repr(&self) -> EcoString {
        eco_format!("styled(child: {}, ..)", self.child.0.repr())
    }
}

/// Hosts metadata and ensures metadata is produced even for empty elements.
#[elem(Behave)]
pub struct MetaElem {
    /// Metadata that should be attached to all elements affected by this style
    /// property.
    #[fold]
    pub data: SmallVec<[Meta; 1]>,
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
pub fn missing_field(field: &str) -> EcoString {
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

/// Makes sure the content is not shared and returns a mutable reference to the
/// inner element.
#[doc(hidden)]
pub fn make_mut(val: &mut Arc<dyn NativeElement>) -> &mut dyn NativeElement {
    if Arc::strong_count(val) > 1 || Arc::weak_count(val) != 0 {
        *val = val.dyn_clone();
    }

    Arc::get_mut(val).unwrap()
}

/// A helper macro to create a field selector used in [`Selector::Elem`]
///
/// Example:
/// ```rs
/// let field_selector = fields!(
///     SequenceElem;
///     SequenceElemFields::Children => vec![]
/// );
/// ```
#[macro_export]
macro_rules! fields {
    ($elem:expr; $($key:expr => $value:expr),* $(,)?) => {{
        use ::typst::model::NativeElement;
        #[allow(unused_mut)]
        let mut out = Vec::<(u8, ::typst::eval::Value)>::new();
        $(
            out.push((
                $elem.field_id($key).unwrap(),
                $value.into_value(),
            ));
        )*
        out
    }};
    ($($key:expr => $value:expr),* $(,)?) => {{
        use ::typst::model::NativeElement;
        #[allow(unused_mut)]
        let mut out = smallvec::SmallVec::<[(u8, ::typst::eval::Value); 1]>::new();
        $(
            out.push((
                $key as u8,
                $value.into_value(),
            ));
        )*
        out
    }};
}
