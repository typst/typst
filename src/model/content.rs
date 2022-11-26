use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use comemo::Tracked;
use siphasher::sip128::{Hasher128, SipHasher};
use typst_macros::node;

use super::{capability, Args, Guard, Key, Property, Recipe, Style, StyleMap, Value, Vm};
use crate::diag::{SourceResult, StrResult};
use crate::syntax::Span;
use crate::util::{EcoString, ReadableTypeId};
use crate::World;

/// Composable representation of styled content.
#[derive(Clone, Hash)]
pub struct Content {
    obj: Arc<dyn Bounds>,
    guards: Vec<Guard>,
    span: Option<Span>,
    label: Option<Label>,
}

impl Content {
    /// Create empty content.
    pub fn empty() -> Self {
        SequenceNode(vec![]).pack()
    }

    /// Create a new sequence node from multiples nodes.
    pub fn sequence(seq: Vec<Self>) -> Self {
        match seq.as_slice() {
            [_] => seq.into_iter().next().unwrap(),
            _ => SequenceNode(seq).pack(),
        }
    }

    /// Attach a span to the content.
    pub fn spanned(mut self, span: Span) -> Self {
        if let Some(styled) = self.to_mut::<StyledNode>() {
            styled.sub.span = Some(span);
        } else if let Some(styled) = self.to::<StyledNode>() {
            self = StyledNode {
                sub: styled.sub.clone().spanned(span),
                map: styled.map.clone(),
            }
            .pack();
        }
        self.span = Some(span);
        self
    }

    /// Attach a label to the content.
    pub fn labelled(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Style this content with a single style property.
    pub fn styled<K: Key>(self, key: K, value: K::Value) -> Self {
        self.styled_with_entry(Style::Property(Property::new(key, value)))
    }

    /// Style this content with a style entry.
    pub fn styled_with_entry(mut self, style: Style) -> Self {
        if let Some(styled) = self.to_mut::<StyledNode>() {
            styled.map.apply_one(style);
            self
        } else if let Some(styled) = self.to::<StyledNode>() {
            let mut map = styled.map.clone();
            map.apply_one(style);
            StyledNode { sub: styled.sub.clone(), map }.pack()
        } else {
            StyledNode { sub: self, map: style.into() }.pack()
        }
    }

    /// Style this content with a full style map.
    pub fn styled_with_map(mut self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            return self;
        }

        if let Some(styled) = self.to_mut::<StyledNode>() {
            styled.map.apply(styles);
            return self;
        }

        StyledNode { sub: self, map: styles }.pack()
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
        (*self.obj).id()
    }

    /// The node's human-readable name.
    pub fn name(&self) -> &'static str {
        (*self.obj).name()
    }

    /// The node's span.
    pub fn span(&self) -> Option<Span> {
        self.span
    }

    /// The content's label.
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Access a field on this content.
    pub fn field(&self, name: &str) -> Option<Value> {
        if name == "label" {
            return Some(match &self.label {
                Some(label) => Value::Label(label.clone()),
                None => Value::None,
            });
        }

        self.obj.field(name)
    }

    /// Whether the contained node is of type `T`.
    pub fn is<T: 'static>(&self) -> bool {
        (*self.obj).as_any().is::<T>()
    }

    /// Cast to `T` if the contained node is of type `T`.
    pub fn to<T: 'static>(&self) -> Option<&T> {
        (*self.obj).as_any().downcast_ref::<T>()
    }

    /// Whether this content has the given capability.
    pub fn has<C>(&self) -> bool
    where
        C: Capability + ?Sized,
    {
        self.obj.vtable(TypeId::of::<C>()).is_some()
    }

    /// Cast to a trait object if this content has the given capability.
    pub fn with<C>(&self) -> Option<&C>
    where
        C: Capability + ?Sized,
    {
        let node: &dyn Bounds = &*self.obj;
        let vtable = node.vtable(TypeId::of::<C>())?;
        let data = node as *const dyn Bounds as *const ();
        Some(unsafe { &*crate::util::fat::from_raw_parts(data, vtable) })
    }

    /// Try to cast to a mutable instance of `T`.
    fn to_mut<T: 'static>(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.obj)?.as_any_mut().downcast_mut::<T>()
    }

    /// Disable a show rule recipe.
    #[doc(hidden)]
    pub fn guarded(mut self, id: Guard) -> Self {
        self.guards.push(id);
        self
    }

    /// Whether a label can be attached to the content.
    pub(super) fn labellable(&self) -> bool {
        !self.has::<dyn Unlabellable>()
    }

    /// Whether no show rule was executed for this node so far.
    pub(super) fn is_pristine(&self) -> bool {
        self.guards.is_empty()
    }

    /// Check whether a show rule recipe is disabled.
    pub(super) fn is_guarded(&self, id: Guard) -> bool {
        self.guards.contains(&id)
    }

    /// Copy the metadata from other content.
    pub(super) fn copy_meta(&mut self, from: &Content) {
        self.guards = from.guards.clone();
        self.span = from.span;
        self.label = from.label.clone();
    }
}

impl Debug for Content {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.obj.fmt(f)
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        (*self.obj).hash128() == (*other.obj).hash128()
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, mut rhs: Self) -> Self::Output {
        let mut lhs = self;
        if let Some(lhs_mut) = lhs.to_mut::<SequenceNode>() {
            if let Some(rhs_mut) = rhs.to_mut::<SequenceNode>() {
                lhs_mut.0.append(&mut rhs_mut.0);
            } else if let Some(rhs) = rhs.to::<SequenceNode>() {
                lhs_mut.0.extend(rhs.0.iter().cloned());
            } else {
                lhs_mut.0.push(rhs);
            }
            return lhs;
        }

        let seq = match (lhs.to::<SequenceNode>(), rhs.to::<SequenceNode>()) {
            (Some(lhs), Some(rhs)) => lhs.0.iter().chain(&rhs.0).cloned().collect(),
            (Some(lhs), None) => lhs.0.iter().cloned().chain(iter::once(rhs)).collect(),
            (None, Some(rhs)) => iter::once(lhs).chain(rhs.0.iter().cloned()).collect(),
            (None, None) => vec![lhs, rhs],
        };

        SequenceNode(seq).pack()
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

trait Bounds: Node + Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn hash128(&self) -> u128;
}

impl<T> Bounds for T
where
    T: Node + Debug + Hash + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn hash128(&self) -> u128 {
        let mut state = SipHasher::new();
        self.type_id().hash(&mut state);
        self.hash(&mut state);
        state.finish128().as_u128()
    }
}

impl Hash for dyn Bounds {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(self.hash128());
    }
}

/// A node with applied styles.
#[derive(Clone, Hash)]
pub struct StyledNode {
    /// The styled content.
    pub sub: Content,
    /// The styles.
    pub map: StyleMap,
}

#[node]
impl StyledNode {}

impl Debug for StyledNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.map.fmt(f)?;
        self.sub.fmt(f)
    }
}

/// A sequence of nodes.
///
/// Combines other arbitrary content. So, when you write `[Hi] + [you]` in
/// Typst, the two text nodes are combined into a single sequence node.
#[derive(Clone, Hash)]
pub struct SequenceNode(pub Vec<Content>);

#[node]
impl SequenceNode {}

impl Debug for SequenceNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_list().entries(self.0.iter()).finish()
    }
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
pub trait Node: 'static + Capable {
    /// Pack a node into type-erased content.
    fn pack(self) -> Content
    where
        Self: Node + Debug + Hash + Sync + Send + Sized + 'static,
    {
        Content {
            obj: Arc::new(self),
            guards: vec![],
            span: None,
            label: None,
        }
    }

    /// A unique identifier of the node type.
    fn id(&self) -> NodeId;

    /// The node's name.
    fn name(&self) -> &'static str;

    /// Construct a node from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// node's set rule.
    fn construct(vm: &Vm, args: &mut Args) -> SourceResult<Content>
    where
        Self: Sized;

    /// Parse relevant arguments into style properties for this node.
    ///
    /// When `constructor` is true, [`construct`](Self::construct) will run
    /// after this invocation of `set` with the remaining arguments.
    fn set(args: &mut Args, constructor: bool) -> SourceResult<StyleMap>
    where
        Self: Sized;

    /// Access a field on this node.
    fn field(&self, name: &str) -> Option<Value>;
}

/// A unique identifier for a node type.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct NodeId(ReadableTypeId);

impl NodeId {
    /// The id of the given node type.
    pub fn of<T: 'static>() -> Self {
        Self(ReadableTypeId::of::<T>())
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A capability a node can have.
///
/// Should be implemented by trait objects that are accessible through
/// [`Capable`].
pub trait Capability: 'static {}

/// Dynamically access a trait implementation at runtime.
pub unsafe trait Capable {
    /// Return the vtable pointer of the trait object with given type `id`
    /// if `self` implements the trait.
    fn vtable(&self, of: TypeId) -> Option<*const ()>;
}

/// Indicates that a node cannot be labelled.
#[capability]
pub trait Unlabellable {}
