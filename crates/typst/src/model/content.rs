use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::iter::{self, Sum};
use std::mem::MaybeUninit;
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use comemo::Prehashed;
use ecow::{eco_format, EcoString, EcoVec};
use serde::{Serialize, Serializer};

use super::{
    elem, Behave, Behaviour, Element, ElementData, Guard, Label, Locatable, Location,
    NativeElement, Recipe, Selector, Style, Styles, Synthesize,
};
use crate::diag::{SourceResult, StrResult};
use crate::doc::Meta;
use crate::eval::{func, scope, ty, Dict, FromValue, IntoValue, Repr, Str, Value, Vm};
use crate::syntax::Span;

#[ty(scope)]
#[derive(Debug, Clone)]
pub enum Content {
    Dyn(DynContent),
    Static(Arc<dyn Element>),
}

impl Content {
    // TODO: remove this.
    pub fn new(elem: ElementData) -> Self {
        Self::dyn_(DynContent::new(elem))
    }
}

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

impl From<DynContent> for Content {
    fn from(value: DynContent) -> Self {
        Self::dyn_(value)
    }
}

impl From<Arc<dyn Element>> for Content {
    fn from(value: Arc<dyn Element>) -> Self {
        Self::Static(value)
    }
}

impl Content {
    #[inline]
    pub fn static_<E: Element>(elem: E) -> Self {
        Self::Static(Arc::new(elem))
    }

    #[inline]
    pub fn dyn_(dyn_: DynContent) -> Self {
        Self::Dyn(dyn_)
    }

    #[inline]
    pub fn empty() -> Self {
        Self::dyn_(DynContent::empty())
    }

    #[inline]
    pub fn get(&self, name: &str) -> StrResult<Value> {
        self.field(name).ok_or_else(|| missing_field(name))
    }

    #[inline]
    pub fn field(&self, name: &str) -> Option<Value> {
        match self {
            Self::Dyn(dyn_) => dyn_.field(name),
            Self::Static(static_) => static_.field(name),
        }
    }

    #[inline]
    pub fn span(&self) -> Span {
        match self {
            Self::Dyn(dyn_) => dyn_.span(),
            Self::Static(static_) => static_.span(),
        }
    }

    #[inline]
    pub fn label(&self) -> Option<&Label> {
        match self {
            Self::Dyn(dyn_) => dyn_.label(),
            Self::Static(static_) => static_.label(),
        }
    }

    pub fn spanned(self, span: Span) -> Self {
        match self {
            Content::Dyn(dyn_) => dyn_.spanned(span).into(),
            Content::Static(mut static_) => {
                static_ = static_.make_mut();
                Arc::get_mut(&mut static_).unwrap().set_span(span);
                static_.into()
            }
        }
    }

    pub fn labelled(self, label: Label) -> Self {
        match self {
            Content::Dyn(dyn_) => dyn_.labelled(label).into(),
            Content::Static(mut static_) => {
                static_ = static_.make_mut();
                Arc::get_mut(&mut static_).unwrap().set_label(label);
                static_.into()
            }
        }
    }

    #[inline]
    pub fn set_location(&mut self, location: Location) {
        match self {
            Self::Dyn(dyn_) => dyn_.set_location(location),
            Self::Static(static_) => {
                swap_with_mut(static_);

                Arc::get_mut(static_).unwrap().set_location(location);
            }
        }
    }

    /// Create a new sequence element from multiples elements.
    pub fn sequence(iter: impl IntoIterator<Item = Self>) -> Self {
        let mut iter = iter.into_iter();
        let Some(first) = iter.next() else { return Self::dyn_(DynContent::empty()) };
        let Some(second) = iter.next() else { return first };
        let mut content = DynContent::empty();
        content.attrs.push(Attr::Child(Prehashed::new(first)));
        content.attrs.push(Attr::Child(Prehashed::new(second)));
        content
            .attrs
            .extend(iter.map(|child| Attr::Child(Prehashed::new(child))));
        Self::dyn_(content)
    }

    /// Access the children if this is a sequence.
    pub fn to_sequence(&self) -> Option<impl Iterator<Item = &Content>> {
        if !self.is_sequence() {
            return None;
        }

        // Todo: make `SequenceElem` a static elem.
        let Self::Dyn(dyn_) = self else {
            return None;
        };

        Some(dyn_.attrs.iter().filter_map(Attr::child))
    }

    pub fn elem(&self) -> ElementData {
        match self {
            Self::Dyn(dyn_) => dyn_.elem,
            Self::Static(static_) => static_.data(),
        }
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
        match self {
            Content::Dyn(dyn_) => {
                let data = dyn_ as *const DynContent as *const ();
                Some(unsafe { &*crate::util::fat::from_raw_parts(data, vtable) })
            }
            Content::Static(static_) => {
                let data = Arc::as_ptr(static_) as *const ();
                Some(unsafe { &*crate::util::fat::from_raw_parts(data, vtable) })
            }
        }
    }

    pub fn with_mut<C>(&mut self) -> Option<&mut C>
    where
        C: ?Sized + 'static,
    {
        let vtable = self.elem().vtable()(TypeId::of::<C>())?;
        match self {
            Content::Dyn(dyn_) => {
                let data = dyn_ as *mut DynContent as *mut ();
                Some(unsafe { &mut *crate::util::fat::from_raw_parts_mut(data, vtable) })
            }
            Content::Static(static_) => {
                swap_with_mut(static_);

                let data = Arc::get_mut(static_).unwrap() as *mut dyn Element as *mut ();
                Some(unsafe { &mut *crate::util::fat::from_raw_parts_mut(data, vtable) })
            }
        }
    }

    pub fn is_sequence(&self) -> bool {
        self.is::<SequenceElem>()
    }

    /// Whether the content is an empty sequence.
    pub fn is_empty(&self) -> bool {
        if !self.is::<SequenceElem>() {
            return false;
        }

        let Self::Dyn(dyn_) = self else {
            return false;
        };

        dyn_.attrs.is_empty()
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
        if !self.is::<StyledElem>() {
            return None;
        }

        let Self::Dyn(dyn_) = self else {
            return None;
        };

        let child = dyn_.attrs.iter().find_map(Attr::child)?;
        let styles = dyn_.attrs.iter().find_map(Attr::styles)?;
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
        Self::sequence(vec![self.clone(); count])
    }

    /// Style this content with a style entry.
    pub fn styled(mut self, style: impl Into<Style>) -> Self {
        if self.is::<StyledElem>() {
            let Self::Dyn(dyn_) = &mut self else { unreachable!() };

            let prev =
                dyn_.attrs.make_mut().iter_mut().find_map(Attr::styles_mut).unwrap();
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
            let Self::Dyn(dyn_) = &mut self else { unreachable!() };

            let prev =
                dyn_.attrs.make_mut().iter_mut().find_map(Attr::styles_mut).unwrap();
            prev.apply(styles);
            self
        } else {
            let mut content = DynContent::new(StyledElem::elem());
            content.attrs.push(Attr::Child(Prehashed::new(self)));
            content.attrs.push(Attr::Styles(styles));
            content.into()
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

    pub fn fields_ref(&self) -> EcoVec<(EcoString, Value)> {
        match self {
            Content::Dyn(dyn_) => dyn_
                .fields_ref()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            Content::Static(static_) => static_
                .fields()
                .into_iter()
                .map(|(key, value)| (key.0, value))
                .collect(),
        }
    }

    /// Traverse this content.
    fn traverse<'a, F>(&'a self, f: &mut F)
    where
        F: FnMut(&'a Content),
    {
        f(self);

        match self {
            Self::Dyn(dyn_) => {
                for attr in &dyn_.attrs {
                    match attr {
                        Attr::Child(child) => child.traverse(f),
                        Attr::Value(value) => walk_value(value, f),
                        _ => {}
                    }
                }
            }
            // TODO: implement children on static elements
            Self::Static(_) => {}
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

    /// Disable a show rule recipe.
    pub fn guarded(self, guard: Guard) -> Self {
        match self {
            Content::Dyn(dyn_) => dyn_.guarded(guard).into(),
            Content::Static(static_) => {
                let mut static_ = static_.make_mut();
                Arc::get_mut(&mut static_).unwrap().push_guard(guard);
                static_.into()
            }
        }
    }

    /// Check whether a show rule recipe is disabled.
    pub fn is_guarded(&self, guard: Guard) -> bool {
        match self {
            Content::Dyn(dyn_) => dyn_.is_guarded(guard),
            Content::Static(static_) => static_.is_guarded(guard),
        }
    }

    /// Whether no show rule was executed for this content so far.
    pub fn is_pristine(&self) -> bool {
        match self {
            Content::Dyn(dyn_) => dyn_.is_pristine(),
            Content::Static(static_) => static_.is_pristine(),
        }
    }

    /// Expect a field on the content to exist as a specified type.
    #[track_caller]
    pub fn expect_field<T: FromValue>(&self, name: &str) -> T {
        self.field(name).unwrap().cast().unwrap()
    }

    /// Whether this content has already been prepared.
    pub fn is_prepared(&self) -> bool {
        match self {
            Content::Dyn(dyn_) => dyn_.is_prepared(),
            Content::Static(static_) => static_.is_prepared(),
        }
    }

    /// Mark this content as prepared.
    pub fn mark_prepared(&mut self) {
        match self {
            Content::Dyn(dyn_) => dyn_.mark_prepared(),
            Content::Static(static_) => {
                swap_with_mut(static_);
                Arc::get_mut(static_).unwrap().mark_prepared();
            }
        }
    }

    /// Attach a field to the content.
    pub fn with_field(self, name: &str, value: impl IntoValue) -> Self {
        match self {
            Content::Dyn(dyn_) => dyn_.with_field(name, value).into(),
            Content::Static(static_) => {
                let mut static_ = static_.make_mut();
                Arc::get_mut(&mut static_)
                    .unwrap()
                    .set_field(name, value.into_value())
                    .unwrap();
                static_.into()
            }
        }
    }
}

impl std::hash::Hash for Content {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);

        match self {
            Content::Dyn(dyn_) => dyn_.hash(state),
            Content::Static(static_) => static_.hash(state),
        }
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Dyn(l0), Self::Dyn(r0)) => l0 == r0,
            (Self::Static(l0), Self::Static(r0)) => l0.eq(r0 as &dyn Any),
            _ => false,
        }
    }
}

impl Repr for Content {
    fn repr(&self) -> EcoString {
        match self {
            Content::Dyn(dyn_) => dyn_.repr(),
            Content::Static(static_) => static_.repr(),
        }
    }
}

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
#[derive(Debug, Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct DynContent {
    elem: ElementData,
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

impl DynContent {
    /// Create an empty element.
    pub fn new(elem: ElementData) -> Self {
        Self { elem, attrs: EcoVec::new() }
    }

    /// Create empty content.
    pub fn empty() -> Self {
        Self::new(SequenceElem::elem())
    }

    pub fn location(&self) -> Option<Location> {
        self.attrs.iter().find_map(Attr::location)
    }

    /// Cast to a mutable trait object if the contained element has the given
    /// capability.
    pub fn with_mut<C>(&mut self) -> Option<&mut C>
    where
        C: ?Sized + 'static,
    {
        let vtable = self.elem.vtable()(TypeId::of::<C>())?;
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

    /// Whether the contained element is of type `T`.
    pub fn is<T: NativeElement>(&self) -> bool {
        self.elem == T::elem()
    }

    pub fn is_sequence(&self) -> bool {
        self.is::<SequenceElem>()
    }

    /// Access the children if this is a sequence.
    pub fn to_sequence(&self) -> Option<impl Iterator<Item = &Content>> {
        if !self.is_sequence() {
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
    pub fn fields_ref(&self) -> impl Iterator<Item = (&EcoString, &Value)> {
        let mut iter = self.attrs.iter();
        std::iter::from_fn(move || {
            let field = iter.find_map(Attr::field)?;
            let value = iter.next()?.value()?;
            Some((field, value))
        })
    }

    /// Borrow the value of the given field.
    pub fn get(&self, key: &str) -> StrResult<Value> {
        self.field(key).ok_or_else(|| missing_field(key))
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

    /// Attach a location to this content.
    pub fn set_location(&mut self, location: Location) {
        self.attrs.push(Attr::Location(location));
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
        match self {
            Self::Dyn(dyn_) => dyn_.attrs.iter().find_map(Attr::location),
            Self::Static(static_) => static_.location(),
        }
    }
}

impl Repr for DynContent {
    fn repr(&self) -> EcoString {
        // TODO: todo!()
        EcoString::new()
        /*let name = self.elem.name();
        // TODO: optimize this.
        if let Some(text) = item!(text_str)(&self.clone().into()) {
            return eco_format!("[{}]", text);
        } else if name == "space" {
            return ("[ ]").into();
        }

        let mut pieces: Vec<_> = self
            .fields()
            .into_iter()
            .map(|(name, value)| eco_format!("{}: {}", name, value.repr()))
            .collect();

        if self.is::<StyledElem>() {
            pieces.push(EcoString::from(".."));
        }

        eco_format!("{}{}", name, pretty_array_like(&pieces, false))*/
    }
}

impl Default for DynContent {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for DynContent {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(left), Some(right)) = (self.to_sequence(), other.to_sequence()) {
            left.eq(right)
        } else if let (Some(left), Some(right)) = (self.to_styled(), other.to_styled()) {
            left == right
        } else {
            self.elem == other.elem && self.fields_ref().eq(other.fields_ref())
        }
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, mut rhs: Self) -> Self::Output {
        let mut lhs = self;
        match (lhs.to_mut::<SequenceElem>(), rhs.to_mut::<SequenceElem>()) {
            (Some(seq_lhs), Some(rhs)) => {
                seq_lhs.0.attrs.extend(rhs.0.attrs.iter().cloned());
                lhs
            }
            (Some(seq_lhs), None) => {
                seq_lhs.0.attrs.push(Attr::Child(Prehashed::new(rhs)));
                lhs
            }
            (None, Some(rhs_seq)) => {
                rhs_seq.0.attrs.insert(0, Attr::Child(Prehashed::new(lhs)));
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
        match self {
            Content::Dyn(dyn_) => serializer.collect_map(
                iter::once((&"func".into(), &dyn_.elem.name().into_value()))
                    .chain(dyn_.fields_ref()),
            ),
            Content::Static(_) => todo!(),
        }
    }
}

impl Attr {
    fn child(&self) -> Option<&Content> {
        match self {
            Self::Child(child) => Some(child),
            _ => None,
        }
    }

    fn location(&self) -> Option<Location> {
        match self {
            Self::Location(location) => Some(*location),
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

/// Defines the `ElemFunc` for sequences.
#[elem]
struct SequenceElem {}

/// Defines the `ElemFunc` for styled elements.
#[elem]
struct StyledElem {}

/// Hosts metadata and ensures metadata is produced even for empty elements.
#[elem(Behave)]
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

#[allow(invalid_value)]
fn swap_with_mut(val: &mut Arc<dyn Element>) {
    // Safety: we forget the old value, so we need to make sure it is not dropped.
    let mut tmp = unsafe { MaybeUninit::uninit().assume_init() };
    std::mem::swap(val, &mut tmp);

    tmp = tmp.make_mut();

    std::mem::swap(val, &mut tmp);
    std::mem::forget(tmp);
}
