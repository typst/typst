mod element;
mod field;
mod packed;
mod raw;
mod vtable;

pub use self::element::*;
pub use self::field::*;
pub use self::packed::Packed;
pub use self::vtable::{ContentVtable, FieldVtable};
#[doc(inline)]
pub use typst_macros::elem;

use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign, ControlFlow};

use comemo::Tracked;
use ecow::{eco_format, EcoString};
use serde::{Serialize, Serializer};

use typst_syntax::Span;
use typst_utils::singleton;

use crate::diag::{SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    func, repr, scope, ty, Context, Dict, IntoValue, Label, Property, Recipe,
    RecipeIndex, Repr, Selector, Str, Style, StyleChain, Styles, Value,
};
use crate::introspection::Location;
use crate::layout::{AlignElem, Alignment, Axes, Length, MoveElem, PadElem, Rel, Sides};
use crate::model::{Destination, EmphElem, LinkElem, StrongElem};
use crate::text::UnderlineElem;

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
/// access syntax when they were explicitly passed to the element function, not
/// when they result from a set rule.
///
/// Each element has a default appearance. However, you can also completely
/// customize its appearance with a [show rule]($styling/#show-rules). The show
/// rule is passed the element. It can access the element's field and produce
/// arbitrary content from it.
///
/// In the web app, you can hover over a content variable to see exactly which
/// elements the content is composed of and what fields they have.
/// Alternatively, you can inspect the output of the [`repr`] function.
#[ty(scope, cast)]
#[derive(Clone, PartialEq, Hash)]
#[repr(transparent)]
pub struct Content(raw::RawContent);

impl Content {
    /// Creates a new content from an element.
    pub fn new<T: NativeElement>(elem: T) -> Self {
        Self(raw::RawContent::new(elem))
    }

    /// Creates a empty sequence content.
    pub fn empty() -> Self {
        singleton!(Content, SequenceElem::default().pack()).clone()
    }

    /// Get the element of this content.
    pub fn elem(&self) -> Element {
        self.0.elem()
    }

    /// Get the span of the content.
    pub fn span(&self) -> Span {
        self.0.span()
    }

    /// Set the span of the content.
    pub fn spanned(mut self, span: Span) -> Self {
        if self.0.span().is_detached() {
            *self.0.span_mut() = span;
        }
        self
    }

    /// Get the label of the content.
    pub fn label(&self) -> Option<Label> {
        self.0.meta().label
    }

    /// Attach a label to the content.
    pub fn labelled(mut self, label: Label) -> Self {
        self.set_label(label);
        self
    }

    /// Set the label of the content.
    pub fn set_label(&mut self, label: Label) {
        self.0.meta_mut().label = Some(label);
    }

    /// Assigns a location to the content.
    ///
    /// This identifies the content and e.g. makes it linkable by
    /// `.linked(Destination::Location(loc))`.
    ///
    /// Useful in combination with [`Location::variant`].
    pub fn located(mut self, loc: Location) -> Self {
        self.set_location(loc);
        self
    }

    /// Set the location of the content.
    pub fn set_location(&mut self, location: Location) {
        self.0.meta_mut().location = Some(location);
    }

    /// Check whether a show rule recipe is disabled.
    pub fn is_guarded(&self, index: RecipeIndex) -> bool {
        self.0.meta().lifecycle.contains(index.0)
    }

    /// Disable a show rule recipe.
    pub fn guarded(mut self, index: RecipeIndex) -> Self {
        self.0.meta_mut().lifecycle.insert(index.0);
        self
    }

    /// Whether this content has already been prepared.
    pub fn is_prepared(&self) -> bool {
        self.0.meta().lifecycle.contains(0)
    }

    /// Mark this content as prepared.
    pub fn mark_prepared(&mut self) {
        self.0.meta_mut().lifecycle.insert(0);
    }

    /// Get a field by ID.
    ///
    /// This is the preferred way to access fields. However, you can only use it
    /// if you have set the field IDs yourself or are using the field IDs
    /// generated by the `#[elem]` macro.
    pub fn get(
        &self,
        id: u8,
        styles: Option<StyleChain>,
    ) -> Result<Value, FieldAccessError> {
        if id == 255
            && let Some(label) = self.label() {
                return Ok(label.into_value());
            }

        match self.0.handle().field(id) {
            Some(handle) => match styles {
                Some(styles) => handle.get_with_styles(styles),
                None => handle.get(),
            }
            .ok_or(FieldAccessError::Unset),
            None => Err(FieldAccessError::Unknown),
        }
    }

    /// Get a field by name.
    ///
    /// If you have access to the field IDs of the element, use [`Self::get`]
    /// instead.
    pub fn get_by_name(&self, name: &str) -> Result<Value, FieldAccessError> {
        if name == "label" {
            return self
                .label()
                .map(|label| label.into_value())
                .ok_or(FieldAccessError::Unknown);
        }

        match self.elem().field_id(name).and_then(|id| self.0.handle().field(id)) {
            Some(handle) => handle.get().ok_or(FieldAccessError::Unset),
            None => Err(FieldAccessError::Unknown),
        }
    }

    /// Get a field by ID, returning a missing field error if it does not exist.
    ///
    /// This is the preferred way to access fields. However, you can only use it
    /// if you have set the field IDs yourself or are using the field IDs
    /// generated by the `#[elem]` macro.
    pub fn field(&self, id: u8) -> StrResult<Value> {
        self.get(id, None)
            .map_err(|e| e.message(self, self.elem().field_name(id).unwrap()))
    }

    /// Get a field by name, returning a missing field error if it does not
    /// exist.
    ///
    /// If you have access to the field IDs of the element, use [`Self::field`]
    /// instead.
    pub fn field_by_name(&self, name: &str) -> StrResult<Value> {
        self.get_by_name(name).map_err(|e| e.message(self, name))
    }

    /// Resolve all fields with the styles and save them in-place.
    pub fn materialize(&mut self, styles: StyleChain) {
        for id in 0..self.elem().vtable().fields.len() as u8 {
            self.0.handle_mut().field(id).unwrap().materialize(styles);
        }
    }

    /// Create a new sequence element from multiples elements.
    pub fn sequence(iter: impl IntoIterator<Item = Self>) -> Self {
        let vec: Vec<_> = iter.into_iter().collect();
        if vec.is_empty() {
            Self::empty()
        } else if vec.len() == 1 {
            vec.into_iter().next().unwrap()
        } else {
            SequenceElem::new(vec).into()
        }
    }

    /// Whether the contained element is of type `T`.
    pub fn is<T: NativeElement>(&self) -> bool {
        self.0.is::<T>()
    }

    /// Downcasts the element to a packed value.
    pub fn to_packed<T: NativeElement>(&self) -> Option<&Packed<T>> {
        Packed::from_ref(self)
    }

    /// Downcasts the element to a mutable packed value.
    pub fn to_packed_mut<T: NativeElement>(&mut self) -> Option<&mut Packed<T>> {
        Packed::from_mut(self)
    }

    /// Downcasts the element into an owned packed value.
    pub fn into_packed<T: NativeElement>(self) -> Result<Packed<T>, Self> {
        Packed::from_owned(self)
    }

    /// Extract the raw underlying element.
    pub fn unpack<T: NativeElement>(self) -> Result<T, Self> {
        self.into_packed::<T>().map(Packed::unpack)
    }

    /// Whether the contained element has the given capability.
    pub fn can<C>(&self) -> bool
    where
        C: ?Sized + 'static,
    {
        self.elem().can::<C>()
    }

    /// Cast to a trait object if the contained element has the given
    /// capability.
    pub fn with<C>(&self) -> Option<&C>
    where
        C: ?Sized + 'static,
    {
        self.0.with::<C>()
    }

    /// Cast to a mutable trait object if the contained element has the given
    /// capability.
    pub fn with_mut<C>(&mut self) -> Option<&mut C>
    where
        C: ?Sized + 'static,
    {
        self.0.with_mut::<C>()
    }

    /// Whether the content is an empty sequence.
    pub fn is_empty(&self) -> bool {
        let Some(sequence) = self.to_packed::<SequenceElem>() else {
            return false;
        };

        sequence.children.is_empty()
    }

    /// Also auto expands sequence of sequences into flat sequence
    pub fn sequence_recursive_for_each<'a>(&'a self, f: &mut impl FnMut(&'a Self)) {
        if let Some(sequence) = self.to_packed::<SequenceElem>() {
            for child in &sequence.children {
                child.sequence_recursive_for_each(f);
            }
        } else {
            f(self);
        }
    }

    /// Style this content with a recipe, eagerly applying it if possible.
    pub fn styled_with_recipe(
        self,
        engine: &mut Engine,
        context: Tracked<Context>,
        recipe: Recipe,
    ) -> SourceResult<Self> {
        if recipe.selector().is_none() {
            recipe.apply(engine, context, self)
        } else {
            Ok(self.styled(recipe))
        }
    }

    /// Repeat this content `count` times.
    pub fn repeat(&self, count: usize) -> Self {
        Self::sequence(std::iter::repeat_with(|| self.clone()).take(count))
    }

    /// Sets a style property on the content.
    pub fn set<E, const I: u8>(self, field: Field<E, I>, value: E::Type) -> Self
    where
        E: SettableProperty<I>,
        E::Type: Debug + Clone + Hash + Send + Sync + 'static,
    {
        self.styled(Property::new(field, value))
    }

    /// Style this content with a style entry.
    pub fn styled(mut self, style: impl Into<Style>) -> Self {
        if let Some(style_elem) = self.to_packed_mut::<StyledElem>() {
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

        if let Some(style_elem) = self.to_packed_mut::<StyledElem>() {
            style_elem.styles.apply(styles);
            self
        } else {
            StyledElem::new(self, styles).into()
        }
    }

    /// Style this content with a full style map in-place.
    pub fn style_in_place(&mut self, styles: Styles) {
        if styles.is_empty() {
            return;
        }

        if let Some(style_elem) = self.to_packed_mut::<StyledElem>() {
            style_elem.styles.apply(styles);
        } else {
            *self = StyledElem::new(std::mem::take(self), styles).into();
        }
    }

    /// Queries the content tree for all elements that match the given selector.
    ///
    /// Elements produced in `show` rules will not be included in the results.
    pub fn query(&self, selector: Selector) -> Vec<Content> {
        let mut results = Vec::new();
        let _ = self.traverse(&mut |element| -> ControlFlow<()> {
            if selector.matches(&element, None) {
                results.push(element);
            }
            ControlFlow::Continue(())
        });
        results
    }

    /// Queries the content tree for the first element that match the given
    /// selector.
    ///
    /// Elements produced in `show` rules will not be included in the results.
    pub fn query_first(&self, selector: &Selector) -> Option<Content> {
        self.traverse(&mut |element| -> ControlFlow<Content> {
            if selector.matches(&element, None) {
                ControlFlow::Break(element)
            } else {
                ControlFlow::Continue(())
            }
        })
        .break_value()
    }

    /// Extracts the plain text of this content.
    pub fn plain_text(&self) -> EcoString {
        let mut text = EcoString::new();
        let _ = self.traverse(&mut |element| -> ControlFlow<()> {
            if let Some(textable) = element.with::<dyn PlainText>() {
                textable.plain_text(&mut text);
            }
            ControlFlow::Continue(())
        });
        text
    }

    /// Traverse this content.
    fn traverse<F, B>(&self, f: &mut F) -> ControlFlow<B>
    where
        F: FnMut(Content) -> ControlFlow<B>,
    {
        /// Walks a given value to find any content that matches the selector.
        ///
        /// Returns early if the function gives `ControlFlow::Break`.
        fn walk_value<F, B>(value: Value, f: &mut F) -> ControlFlow<B>
        where
            F: FnMut(Content) -> ControlFlow<B>,
        {
            match value {
                Value::Content(content) => content.traverse(f),
                Value::Array(array) => {
                    for value in array {
                        walk_value(value, f)?;
                    }
                    ControlFlow::Continue(())
                }
                _ => ControlFlow::Continue(()),
            }
        }

        // Call f on the element itself before recursively iterating its fields.
        f(self.clone())?;
        for (_, value) in self.fields() {
            walk_value(value, f)?;
        }
        ControlFlow::Continue(())
    }
}

impl Content {
    /// Strongly emphasize this content.
    pub fn strong(self) -> Self {
        let span = self.span();
        StrongElem::new(self).pack().spanned(span)
    }

    /// Emphasize this content.
    pub fn emph(self) -> Self {
        let span = self.span();
        EmphElem::new(self).pack().spanned(span)
    }

    /// Underline this content.
    pub fn underlined(self) -> Self {
        let span = self.span();
        UnderlineElem::new(self).pack().spanned(span)
    }

    /// Link the content somewhere.
    pub fn linked(self, dest: Destination) -> Self {
        self.set(LinkElem::current, Some(dest))
    }

    /// Set alignments for this content.
    pub fn aligned(self, align: Alignment) -> Self {
        self.set(AlignElem::alignment, align)
    }

    /// Pad this content at the sides.
    pub fn padded(self, padding: Sides<Rel<Length>>) -> Self {
        let span = self.span();
        PadElem::new(self)
            .with_left(padding.left)
            .with_top(padding.top)
            .with_right(padding.right)
            .with_bottom(padding.bottom)
            .pack()
            .spanned(span)
    }

    /// Transform this content's contents without affecting layout.
    pub fn moved(self, delta: Axes<Rel<Length>>) -> Self {
        let span = self.span();
        MoveElem::new(self)
            .with_dx(delta.x)
            .with_dy(delta.y)
            .pack()
            .spanned(span)
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
        if field.as_str() == "label" {
            return self.label().is_some();
        }

        let Some(id) = self.elem().field_id(&field) else {
            return false;
        };

        match self.0.handle().field(id) {
            Some(field) => field.has(),
            None => false,
        }
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
        self.get_by_name(&field)
            .or_else(|e| default.ok_or(e))
            .map_err(|e| e.message_no_default(self, &field))
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
        let mut dict = Dict::new();
        for field in self.0.handle().fields() {
            if let Some(value) = field.get() {
                dict.insert(field.name.into(), value);
            }
        }
        if let Some(label) = self.label() {
            dict.insert("label".into(), label.into_value());
        }
        dict
    }

    /// The location of the content. This is only available on content returned
    /// by [query] or provided by a [show rule]($reference/styling/#show-rules),
    /// for other content it will be `{none}`. The resulting location can be
    /// used with [counters]($counter), [state] and [queries]($query).
    #[func]
    pub fn location(&self) -> Option<Location> {
        self.0.meta().location
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::empty()
    }
}

impl Debug for Content {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: NativeElement> From<T> for Content {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl Repr for Content {
    fn repr(&self) -> EcoString {
        self.0.handle().repr().unwrap_or_else(|| {
            let fields = self
                .0
                .handle()
                .fields()
                .filter_map(|field| field.get().map(|v| (field.name, v.repr())))
                .map(|(name, value)| eco_format!("{name}: {value}"))
                .collect::<Vec<_>>();
            eco_format!(
                "{}{}",
                self.elem().name(),
                repr::pretty_array_like(&fields, false),
            )
        })
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, mut rhs: Self) -> Self::Output {
        let mut lhs = self;
        match (lhs.to_packed_mut::<SequenceElem>(), rhs.to_packed_mut::<SequenceElem>()) {
            (Some(seq_lhs), Some(rhs)) => {
                seq_lhs.children.extend(rhs.children.iter().cloned());
                lhs
            }
            (Some(seq_lhs), None) => {
                seq_lhs.children.push(rhs);
                lhs
            }
            (None, Some(rhs_seq)) => {
                rhs_seq.children.insert(0, lhs);
                rhs
            }
            (None, None) => Self::sequence([lhs, rhs]),
        }
    }
}

impl<'a> Add<&'a Self> for Content {
    type Output = Self;

    fn add(self, rhs: &'a Self) -> Self::Output {
        let mut lhs = self;
        match (lhs.to_packed_mut::<SequenceElem>(), rhs.to_packed::<SequenceElem>()) {
            (Some(seq_lhs), Some(rhs)) => {
                seq_lhs.children.extend(rhs.children.iter().cloned());
                lhs
            }
            (Some(seq_lhs), None) => {
                seq_lhs.children.push(rhs.clone());
                lhs
            }
            (None, Some(_)) => {
                let mut rhs = rhs.clone();
                rhs.to_packed_mut::<SequenceElem>().unwrap().children.insert(0, lhs);
                rhs
            }
            (None, None) => Self::sequence([lhs, rhs.clone()]),
        }
    }
}

impl AddAssign for Content {
    fn add_assign(&mut self, rhs: Self) {
        *self = std::mem::take(self) + rhs;
    }
}

impl AddAssign<&Self> for Content {
    fn add_assign(&mut self, rhs: &Self) {
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
            iter::once(("func".into(), self.func().name().into_value()))
                .chain(self.fields()),
        )
    }
}

/// A sequence of content.
#[elem(Debug, Repr)]
pub struct SequenceElem {
    /// The elements.
    #[required]
    pub children: Vec<Content>,
}

impl Debug for SequenceElem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Sequence ")?;
        f.debug_list().entries(&self.children).finish()
    }
}

// Derive is currently incompatible with `elem` macro.
#[allow(clippy::derivable_impls)]
impl Default for SequenceElem {
    fn default() -> Self {
        Self { children: Default::default() }
    }
}

impl Repr for SequenceElem {
    fn repr(&self) -> EcoString {
        if self.children.is_empty() {
            "[]".into()
        } else {
            let elements = crate::foundations::repr::pretty_array_like(
                &self.children.iter().map(|c| c.repr()).collect::<Vec<_>>(),
                false,
            );
            eco_format!("sequence{}", elements)
        }
    }
}

/// Content alongside styles.
#[elem(Debug, Repr, PartialEq)]
pub struct StyledElem {
    /// The content.
    #[required]
    pub child: Content,
    /// The styles.
    #[required]
    pub styles: Styles,
}

impl Debug for StyledElem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for style in self.styles.iter() {
            writeln!(f, "#{style:?}")?;
        }
        self.child.fmt(f)
    }
}

impl PartialEq for StyledElem {
    fn eq(&self, other: &Self) -> bool {
        self.child == other.child
    }
}

impl Repr for StyledElem {
    fn repr(&self) -> EcoString {
        eco_format!("styled(child: {}, ..)", self.child.repr())
    }
}

impl<T: NativeElement> IntoValue for T {
    fn into_value(self) -> Value {
        Value::Content(self.pack())
    }
}
