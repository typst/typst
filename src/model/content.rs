use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use comemo::Tracked;
use ecow::{EcoString, EcoVec};
use siphasher::sip128::{Hasher128, SipHasher};
use typst_macros::node;

use super::{
    capability, capable, Args, Guard, Key, ParamInfo, Property, Recipe, Style, StyleMap,
    Value, Vm,
};
use crate::diag::{SourceResult, StrResult};
use crate::syntax::Span;
use crate::util::ReadableTypeId;
use crate::World;

/// Composable representation of styled content.
#[derive(Clone, Hash)]
pub struct Content {
    obj: Arc<dyn Bounds>,
    span: Option<Span>,
    modifiers: EcoVec<Modifier>,
}

/// Modifiers that can be attached to content.
#[derive(Debug, Clone, PartialEq, Hash)]
enum Modifier {
    Prepared,
    Guard(Guard),
    Label(Label),
    Field(EcoString, Value),
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
        for (i, modifier) in self.modifiers.iter().enumerate() {
            if matches!(modifier, Modifier::Label(_)) {
                self.modifiers.make_mut()[i] = Modifier::Label(label);
                return self;
            }
        }

        self.modifiers.push(Modifier::Label(label));
        self
    }

    /// Attach a field to the content.
    pub fn push_field(&mut self, name: impl Into<EcoString>, value: Value) {
        self.modifiers.push(Modifier::Field(name.into(), value));
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
        self.modifiers.iter().find_map(|modifier| match modifier {
            Modifier::Label(label) => Some(label),
            _ => None,
        })
    }

    /// Access a field on this content.
    pub fn field(&self, name: &str) -> Option<Value> {
        if name == "label" {
            return Some(match self.label() {
                Some(label) => Value::Label(label.clone()),
                None => Value::None,
            });
        }

        for modifier in &self.modifiers {
            if let Modifier::Field(other, value) = modifier {
                if name == other {
                    return Some(value.clone());
                }
            }
        }

        self.obj.field(name)
    }

    /// Whether the contained node is of type `T`.
    pub fn is<T>(&self) -> bool
    where
        T: Capable + 'static,
    {
        (*self.obj).as_any().is::<T>()
    }

    /// Cast to `T` if the contained node is of type `T`.
    pub fn to<T>(&self) -> Option<&T>
    where
        T: Capable + 'static,
    {
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
    pub(super) fn labellable(&self) -> bool {
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
#[capable]
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
#[capable]
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
            span: None,
            modifiers: EcoVec::new(),
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

    /// List the settable properties.
    fn properties() -> Vec<ParamInfo>
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
