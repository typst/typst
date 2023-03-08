use std::any::TypeId;
use std::fmt::{self, Debug, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign};

use comemo::Tracked;
use ecow::{EcoString, EcoVec};

use super::{node, Guard, Key, Property, Recipe, Style, StyleMap};
use crate::diag::{SourceResult, StrResult};
use crate::eval::{cast_from_value, Args, Cast, ParamInfo, Value, Vm};
use crate::syntax::Span;
use crate::World;

/// Composable representation of styled content.
#[derive(Clone, Hash)]
pub struct Content {
    id: NodeId,
    span: Option<Span>,
    fields: EcoVec<(EcoString, Value)>,
    modifiers: EcoVec<Modifier>,
}

/// Modifiers that can be attached to content.
#[derive(Debug, Clone, PartialEq, Hash)]
enum Modifier {
    Prepared,
    Guard(Guard),
}

impl Content {
    pub fn new<T: Node>() -> Self {
        Self {
            id: T::id(),
            span: None,
            fields: EcoVec::new(),
            modifiers: EcoVec::new(),
        }
    }

    /// Create empty content.
    pub fn empty() -> Self {
        SequenceNode::new(vec![]).pack()
    }

    /// Create a new sequence node from multiples nodes.
    pub fn sequence(seq: Vec<Self>) -> Self {
        match seq.as_slice() {
            [_] => seq.into_iter().next().unwrap(),
            _ => SequenceNode::new(seq).pack(),
        }
    }

    /// Attach a span to the content.
    pub fn spanned(mut self, span: Span) -> Self {
        if let Some(styled) = self.to::<StyledNode>() {
            self = StyledNode::new(styled.sub().spanned(span), styled.map()).pack();
        }
        self.span = Some(span);
        self
    }

    /// Attach a label to the content.
    pub fn labelled(self, label: Label) -> Self {
        self.with_field("label", label)
    }

    /// Style this content with a single style property.
    pub fn styled<K: Key>(self, key: K, value: K::Value) -> Self {
        self.styled_with_entry(Style::Property(Property::new(key, value)))
    }

    /// Style this content with a style entry.
    pub fn styled_with_entry(self, style: Style) -> Self {
        self.styled_with_map(style.into())
    }

    /// Style this content with a full style map.
    pub fn styled_with_map(self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            self
        } else if let Some(styled) = self.to::<StyledNode>() {
            let mut map = styled.map();
            map.apply(styles);
            StyledNode::new(styled.sub(), map).pack()
        } else {
            StyledNode::new(self, styles).pack()
        }
    }

    /// Style this content with a recipe, eagerly applying it if possible.
    pub fn styled_with_recipe(
        self,
        world: Tracked<dyn World>,
        recipe: Recipe,
    ) -> SourceResult<Self> {
        if recipe.selector.is_none() {
            recipe.apply(world, self)
        } else {
            Ok(self.styled_with_entry(Style::Recipe(recipe)))
        }
    }

    /// Repeat this content `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this content {} times", n))?;

        Ok(Self::sequence(vec![self.clone(); count]))
    }
}

impl Content {
    /// The id of the contained node.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// The node's human-readable name.
    pub fn name(&self) -> &'static str {
        self.id.name()
    }

    /// The node's span.
    pub fn span(&self) -> Option<Span> {
        self.span
    }

    /// The content's label.
    pub fn label(&self) -> Option<&Label> {
        match self.field("label")? {
            Value::Label(label) => Some(label),
            _ => None,
        }
    }

    pub fn with_field(
        mut self,
        name: impl Into<EcoString>,
        value: impl Into<Value>,
    ) -> Self {
        self.push_field(name, value);
        self
    }

    /// Attach a field to the content.
    pub fn push_field(&mut self, name: impl Into<EcoString>, value: impl Into<Value>) {
        let name = name.into();
        if let Some(i) = self.fields.iter().position(|(field, _)| *field == name) {
            self.fields.make_mut()[i] = (name, value.into());
        } else {
            self.fields.push((name, value.into()));
        }
    }

    pub fn field(&self, name: &str) -> Option<&Value> {
        static NONE: Value = Value::None;
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
            .or_else(|| (name == "label").then(|| &NONE))
    }

    pub fn fields(&self) -> &[(EcoString, Value)] {
        &self.fields
    }

    #[track_caller]
    pub fn cast_field<T: Cast>(&self, name: &str) -> T {
        match self.field(name) {
            Some(value) => value.clone().cast().unwrap(),
            None => field_is_missing(name),
        }
    }

    /// Whether the contained node is of type `T`.
    pub fn is<T>(&self) -> bool
    where
        T: Node + 'static,
    {
        self.id == NodeId::of::<T>()
    }

    /// Cast to `T` if the contained node is of type `T`.
    pub fn to<T>(&self) -> Option<&T>
    where
        T: Node + 'static,
    {
        self.is::<T>().then(|| unsafe { std::mem::transmute(self) })
    }

    /// Whether this content has the given capability.
    pub fn has<C>(&self) -> bool
    where
        C: ?Sized + 'static,
    {
        (self.id.0.vtable)(TypeId::of::<C>()).is_some()
    }

    /// Cast to a trait object if this content has the given capability.
    pub fn with<C>(&self) -> Option<&C>
    where
        C: ?Sized + 'static,
    {
        let vtable = (self.id.0.vtable)(TypeId::of::<C>())?;
        let data = self as *const Self as *const ();
        Some(unsafe { &*crate::util::fat::from_raw_parts(data, vtable) })
    }

    /// Disable a show rule recipe.
    #[doc(hidden)]
    pub fn guarded(mut self, id: Guard) -> Self {
        self.modifiers.push(Modifier::Guard(id));
        self
    }

    /// Mark this content as prepared.
    #[doc(hidden)]
    pub fn prepared(mut self) -> Self {
        self.modifiers.push(Modifier::Prepared);
        self
    }

    /// Whether this node was prepared.
    #[doc(hidden)]
    pub fn is_prepared(&self) -> bool {
        self.modifiers.contains(&Modifier::Prepared)
    }

    /// Whether a label can be attached to the content.
    pub(crate) fn labellable(&self) -> bool {
        !self.has::<dyn Unlabellable>()
    }

    /// Whether no show rule was executed for this node so far.
    pub(super) fn is_pristine(&self) -> bool {
        !self
            .modifiers
            .iter()
            .any(|modifier| matches!(modifier, Modifier::Guard(_)))
    }

    /// Check whether a show rule recipe is disabled.
    pub(super) fn is_guarded(&self, id: Guard) -> bool {
        self.modifiers.contains(&Modifier::Guard(id))
    }

    /// Copy the modifiers from another piece of content.
    pub(super) fn copy_modifiers(&mut self, from: &Content) {
        self.span = from.span;
        self.modifiers = from.modifiers.clone();
        if let Some(label) = from.label() {
            self.push_field("label", label.clone())
        }
    }
}

impl Debug for Content {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        struct Pad<'a>(&'a str);
        impl Debug for Pad<'_> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                f.pad(self.0)
            }
        }

        if let Some(styled) = self.to::<StyledNode>() {
            styled.map().fmt(f)?;
            styled.sub().fmt(f)
        } else if let Some(seq) = self.to::<SequenceNode>() {
            f.debug_list().entries(&seq.children()).finish()
        } else if self.id.name() == "space" {
            ' '.fmt(f)
        } else if self.id.name() == "text" {
            self.field("text").unwrap().fmt(f)
        } else {
            f.write_str(self.name())?;
            if self.fields.is_empty() {
                return Ok(());
            }
            f.write_char(' ')?;
            f.debug_map()
                .entries(self.fields.iter().map(|(name, value)| (Pad(name), value)))
                .finish()
        }
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::empty()
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let lhs = self;
        let seq = match (lhs.to::<SequenceNode>(), rhs.to::<SequenceNode>()) {
            (Some(lhs), Some(rhs)) => {
                lhs.children().into_iter().chain(rhs.children()).collect()
            }
            (Some(lhs), None) => {
                lhs.children().into_iter().chain(iter::once(rhs)).collect()
            }
            (None, Some(rhs)) => iter::once(lhs).chain(rhs.children()).collect(),
            (None, None) => vec![lhs, rhs],
        };
        SequenceNode::new(seq).pack()
    }
}

impl AddAssign for Content {
    fn add_assign(&mut self, rhs: Self) {
        *self = std::mem::take(self) + rhs;
    }
}

impl Sum for Content {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::sequence(iter.collect())
    }
}

/// A node with applied styles.
///
/// Display: Styled
/// Category: special
#[node]
pub struct StyledNode {
    /// The styled content.
    #[positional]
    #[required]
    pub sub: Content,

    /// The styles.
    #[positional]
    #[required]
    pub map: StyleMap,
}

cast_from_value! {
    StyleMap: "style map",
}

/// A sequence of nodes.
///
/// Combines other arbitrary content. So, when you write `[Hi] + [you]` in
/// Typst, the two text nodes are combined into a single sequence node.
///
/// Display: Sequence
/// Category: special
#[node]
pub struct SequenceNode {
    #[variadic]
    #[required]
    pub children: Vec<Content>,
}

/// A label for a node.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Label(pub EcoString);

impl Debug for Label {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

/// A constructable, stylable content node.
pub trait Node: Construct + Set + Sized + 'static {
    /// The node's ID.
    fn id() -> NodeId;

    /// Pack a node into type-erased content.
    fn pack(self) -> Content;
}

/// A unique identifier for a node.
#[derive(Copy, Clone)]
pub struct NodeId(&'static NodeMeta);

impl NodeId {
    pub fn of<T: Node>() -> Self {
        T::id()
    }

    pub fn from_meta(meta: &'static NodeMeta) -> Self {
        Self(meta)
    }

    /// The name of the identified node.
    pub fn name(self) -> &'static str {
        self.0.name
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.name())
    }
}

impl Hash for NodeId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as *const _ as usize);
    }
}

impl Eq for NodeId {}

impl PartialEq for NodeId {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct NodeMeta {
    pub name: &'static str,
    pub vtable: fn(of: TypeId) -> Option<*const ()>,
}

pub trait Construct {
    /// Construct a node from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// node's set rule.
    fn construct(vm: &Vm, args: &mut Args) -> SourceResult<Content>;
}

pub trait Set {
    /// Parse relevant arguments into style properties for this node.
    ///
    /// When `constructor` is true, [`construct`](Construct::construct) will run
    /// after this invocation of `set` with the remaining arguments.
    fn set(args: &mut Args, constructor: bool) -> SourceResult<StyleMap>;

    /// List the settable properties.
    fn properties() -> Vec<ParamInfo>;
}

/// Indicates that a node cannot be labelled.
pub trait Unlabellable {}

#[cold]
#[track_caller]
fn field_is_missing(name: &str) -> ! {
    panic!("required field `{name}` is missing")
}
