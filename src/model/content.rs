use std::any::TypeId;
use std::fmt::{self, Debug, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign, Deref};

use comemo::Tracked;
use ecow::{eco_format, EcoString, EcoVec};
use once_cell::sync::Lazy;

use super::{node, Guard, Recipe, Style, StyleMap};
use crate::diag::{SourceResult, StrResult};
use crate::eval::{cast_from_value, Args, FuncInfo, Str, Value, Vm};
use crate::syntax::Span;
use crate::util::pretty_array;
use crate::World;

/// Composable representation of styled content.
#[derive(Clone, Hash)]
pub struct Content {
    id: NodeId,
    span: Span,
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
            span: Span::detached(),
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

    /// The id of the contained node.
    pub fn id(&self) -> NodeId {
        self.id
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
    pub fn can<C>(&self) -> bool
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

    /// The node's span.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Attach a span to the content.
    pub fn spanned(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    /// Access a field on the content.
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    }

    /// List all fields on the content.
    pub fn fields(&self) -> &[(EcoString, Value)] {
        &self.fields
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

    /// Attach a field to the content.
    pub fn push_field(&mut self, name: impl Into<EcoString>, value: impl Into<Value>) {
        let name = name.into();
        if let Some(i) = self.fields.iter().position(|(field, _)| *field == name) {
            self.fields.make_mut()[i] = (name, value.into());
        } else {
            self.fields.push((name, value.into()));
        }
    }

    /// Whether the content has the specified field.
    pub fn has(&self, field: &str) -> bool {
        self.field(field).is_some()
    }

    /// Borrow the value of the given field.
    pub fn at(&self, field: &str) -> StrResult<&Value> {
        self.field(field).ok_or_else(|| missing_field(field))
    }

    /// The content's label.
    pub fn label(&self) -> Option<&Label> {
        match self.field("label")? {
            Value::Label(label) => Some(label),
            _ => None,
        }
    }

    /// Attach a label to the content.
    pub fn labelled(self, label: Label) -> Self {
        self.with_field("label", label)
    }

    /// Style this content with a style entry.
    pub fn styled(self, style: impl Into<Style>) -> Self {
        self.styled_with_map(style.into().into())
    }

    /// Style this content with a full style map.
    pub fn styled_with_map(self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            self
        } else if let Some(styled) = self.to::<StyledNode>() {
            let mut map = styled.styles();
            map.apply(styles);
            StyledNode::new(map, styled.body()).pack()
        } else {
            StyledNode::new(styles, self).pack()
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
            Ok(self.styled(Style::Recipe(recipe)))
        }
    }

    /// Repeat this content `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this content {} times", n))?;

        Ok(Self::sequence(vec![self.clone(); count]))
    }
}

#[doc(hidden)]
impl Content {
    /// Disable a show rule recipe.
    pub fn guarded(mut self, id: Guard) -> Self {
        self.modifiers.push(Modifier::Guard(id));
        self
    }

    /// Mark this content as prepared.
    pub fn prepared(mut self) -> Self {
        self.modifiers.push(Modifier::Prepared);
        self
    }

    /// Whether this node was prepared.
    pub fn is_prepared(&self) -> bool {
        self.modifiers.contains(&Modifier::Prepared)
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
        let name = self.id.name;
        if let Some(text) = item!(text_str)(self) {
            f.write_char('[')?;
            f.write_str(&text)?;
            f.write_char(']')?;
            return Ok(());
        } else if name == "space" {
            return f.write_str("[ ]");
        }

        let pieces: Vec<_> = self
            .fields
            .iter()
            .map(|(name, value)| eco_format!("{name}: {value:?}"))
            .collect();

        f.write_str(name)?;
        f.write_str(&pretty_array(&pieces, false))
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.fields.len() == other.fields.len()
            && self
                .fields
                .iter()
                .all(|(name, value)| other.field(name) == Some(value))
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

/// A constructable, stylable content node.
pub trait Node: Construct + Set + Sized + 'static {
    /// The node's ID.
    fn id() -> NodeId;

    /// Pack a node into type-erased content.
    fn pack(self) -> Content;
}

/// A unique identifier for a node.
#[derive(Copy, Clone)]
pub struct NodeId(pub &'static NodeMeta);

impl NodeId {
    /// Get the id of a node.
    pub fn of<T: Node>() -> Self {
        T::id()
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.name)
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

impl Deref for NodeId {
    type Target = NodeMeta;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Static node for a node.
pub struct NodeMeta {
    /// The node's name.
    pub name: &'static str,
    /// The node's vtable for caspability dispatch.
    pub vtable: fn(of: TypeId) -> Option<*const ()>,
    /// The node's constructor.
    pub construct: fn(&Vm, &mut Args) -> SourceResult<Content>,
    /// The node's set rule.
    pub set: fn(&mut Args) -> SourceResult<StyleMap>,
    /// Details about the function.
    pub info: Lazy<FuncInfo>,
}

/// A node's constructor function.
pub trait Construct {
    /// Construct a node from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// node's set rule.
    fn construct(vm: &Vm, args: &mut Args) -> SourceResult<Content>;
}

/// A node's set rule.
pub trait Set {
    /// Parse relevant arguments into style properties for this node.
    fn set(args: &mut Args) -> SourceResult<StyleMap>;
}

/// Indicates that a node cannot be labelled.
pub trait Unlabellable {}

/// A label for a node.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Label(pub EcoString);

impl Debug for Label {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
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
    pub children: Vec<Content>,
}

/// A node with applied styles.
///
/// Display: Styled
/// Category: special
#[node]
pub struct StyledNode {
    /// The styles.
    #[required]
    pub styles: StyleMap,

    /// The styled content.
    #[required]
    pub body: Content,
}

cast_from_value! {
    StyleMap: "style map",
}

/// The missing key access error message.
#[cold]
#[track_caller]
fn missing_field(key: &str) -> EcoString {
    eco_format!("content does not contain field {:?}", Str::from(key))
}
