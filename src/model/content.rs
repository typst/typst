use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{self, Sum};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use comemo::Tracked;
use siphasher::sip128::{Hasher128, SipHasher};
use typst_macros::node;

use super::{
    Args, Barrier, Builder, Key, Layout, Level, Property, Regions, Scratch, Selector,
    StyleChain, StyleEntry, StyleMap, Vm,
};
use crate::diag::{SourceResult, StrResult};
use crate::frame::Frame;
use crate::util::ReadableTypeId;
use crate::World;

/// Composable representation of styled content.
///
/// This results from:
/// - anything written between square brackets in Typst
/// - any constructor function
#[derive(Clone, Hash)]
pub struct Content(Arc<dyn Bounds>);

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

    pub fn is_empty(&self) -> bool {
        self.downcast::<SequenceNode>().map_or(false, |seq| seq.0.is_empty())
    }

    pub fn id(&self) -> NodeId {
        (*self.0).id()
    }

    pub fn is<T: 'static>(&self) -> bool {
        (*self.0).as_any().is::<T>()
    }

    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        (*self.0).as_any().downcast_ref::<T>()
    }

    fn try_downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.0)?.as_any_mut().downcast_mut::<T>()
    }

    /// Whether this content has the given capability.
    pub fn has<C>(&self) -> bool
    where
        C: Capability + ?Sized,
    {
        self.0.vtable(TypeId::of::<C>()).is_some()
    }

    /// Cast to a trait object if this content has the given capability.
    pub fn to<C>(&self) -> Option<&C>
    where
        C: Capability + ?Sized,
    {
        let node: &dyn Bounds = &*self.0;
        let vtable = node.vtable(TypeId::of::<C>())?;
        let data = node as *const dyn Bounds as *const ();
        Some(unsafe { &*crate::util::fat::from_raw_parts(data, vtable) })
    }

    /// Repeat this content `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this content {} times", n))?;

        Ok(Self::sequence(vec![self.clone(); count]))
    }

    /// Style this content with a single style property.
    pub fn styled<'k, K: Key<'k>>(self, key: K, value: K::Value) -> Self {
        self.styled_with_entry(StyleEntry::Property(Property::new(key, value)))
    }

    /// Style this content with a style entry.
    pub fn styled_with_entry(mut self, entry: StyleEntry) -> Self {
        if let Some(styled) = self.try_downcast_mut::<StyledNode>() {
            styled.map.apply(entry);
            return self;
        }

        StyledNode { sub: self, map: entry.into() }.pack()
    }

    /// Style this content with a full style map.
    pub fn styled_with_map(mut self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            return self;
        }

        if let Some(styled) = self.try_downcast_mut::<StyledNode>() {
            styled.map.apply_map(&styles);
            return self;
        }

        StyledNode { sub: self, map: styles }.pack()
    }

    /// Reenable the show rule identified by the selector.
    pub fn unguard(&self, sel: Selector) -> Self {
        self.clone().styled_with_entry(StyleEntry::Unguard(sel))
    }

    #[comemo::memoize]
    pub fn layout_block(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let barrier = StyleEntry::Barrier(Barrier::new(self.id()));
        let styles = barrier.chain(&styles);

        if let Some(node) = self.to::<dyn Layout>() {
            if node.level() == Level::Block {
                return node.layout(world, regions, styles);
            }
        }

        let scratch = Scratch::default();
        let mut builder = Builder::new(world, &scratch, false);
        builder.accept(self, styles)?;
        let (flow, shared) = builder.into_flow(styles)?;
        flow.layout(world, regions, shared)
    }


    #[comemo::memoize]
    pub fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let barrier = StyleEntry::Barrier(Barrier::new(self.id()));
        let styles = barrier.chain(&styles);

        if let Some(node) = self.to::<dyn Layout>() {
            return node.layout(world, regions, styles);
        }

        let scratch = Scratch::default();
        let mut builder = Builder::new(world, &scratch, false);
        builder.accept(self, styles)?;
        let (flow, shared) = builder.into_flow(styles)?;
        flow.layout(world, regions, shared)
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

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        (*self.0).hash128() == (*other.0).hash128()
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, mut rhs: Self) -> Self::Output {
        let mut lhs = self;
        if let Some(lhs_mut) = lhs.try_downcast_mut::<SequenceNode>() {
            if let Some(rhs_mut) = rhs.try_downcast_mut::<SequenceNode>() {
                lhs_mut.0.extend(rhs_mut.0.drain(..));
            } else if let Some(rhs) = rhs.downcast::<SequenceNode>() {
                lhs_mut.0.extend(rhs.0.iter().cloned());
            } else {
                lhs_mut.0.push(rhs);
            }
            return lhs;
        }

        let seq = match (
            lhs.downcast::<SequenceNode>(),
            rhs.downcast::<SequenceNode>(),
        ) {
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

/// A constructable, stylable content node.
pub trait Node: 'static {
    /// Pack into type-erased content.
    fn pack(self) -> Content
    where
        Self: Debug + Hash + Sync + Send + Sized + 'static,
    {
        Content(Arc::new(self))
    }

    /// Construct a node from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// node's set rule.
    fn construct(vm: &mut Vm, args: &mut Args) -> SourceResult<Content>
    where
        Self: Sized;

    /// Parse relevant arguments into style properties for this node.
    ///
    /// When `constructor` is true, [`construct`](Self::construct) will run
    /// after this invocation of `set` with the remaining arguments.
    fn set(args: &mut Args, constructor: bool) -> SourceResult<StyleMap>
    where
        Self: Sized;

    /// A unique identifier of the node type.
    fn id(&self) -> NodeId;

    /// Extract the pointer of the vtable of the trait object with the
    /// given type `id` if this node implements that trait.
    fn vtable(&self, id: TypeId) -> Option<*const ()>;
}

/// A unique identifier for a node.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct NodeId(ReadableTypeId);

impl NodeId {
    /// The id of the given node.
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
/// This is implemented by trait objects.
pub trait Capability: 'static + Send + Sync {}

/// A node with applied styles.
#[derive(Clone, Hash)]
pub struct StyledNode {
    pub sub: Content,
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
