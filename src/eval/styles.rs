use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

// Possible optimizations:
// - Ref-count map for cheaper cloning and smaller footprint
// - Store map in `Option` to make empty maps non-allocating
// - Store small properties inline

/// A map of style properties.
#[derive(Default, Clone, Hash)]
pub struct Styles {
    map: Vec<(StyleId, Entry)>,
}

impl Styles {
    /// Create a new, empty style map.
    pub fn new() -> Self {
        Self { map: vec![] }
    }

    /// Create a style map with a single property-value pair.
    pub fn one<P: Property>(key: P, value: P::Value) -> Self {
        let mut styles = Self::new();
        styles.set(key, value);
        styles
    }

    /// Whether this map contains no styles.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Set the value for a style property.
    pub fn set<P: Property>(&mut self, key: P, value: P::Value) {
        let id = StyleId::of::<P>();
        for pair in &mut self.map {
            if pair.0 == id {
                let prev = pair.1.downcast::<P::Value>().unwrap();
                let folded = P::combine(value, prev.clone());
                pair.1 = Entry::new(key, folded);
                return;
            }
        }

        self.map.push((id, Entry::new(key, value)));
    }

    /// Toggle a boolean style property.
    pub fn toggle<P: Property<Value = bool>>(&mut self, key: P) {
        let id = StyleId::of::<P>();
        for (i, pair) in self.map.iter_mut().enumerate() {
            if pair.0 == id {
                self.map.swap_remove(i);
                return;
            }
        }

        self.map.push((id, Entry::new(key, true)));
    }

    /// Get the value of a copyable style property.
    ///
    /// Returns the property's default value if the map does not contain an
    /// entry for it.
    pub fn get<P: Property>(&self, key: P) -> P::Value
    where
        P::Value: Copy,
    {
        self.get_direct(key).copied().unwrap_or_else(P::default)
    }

    /// Get a reference to a style property.
    ///
    /// Returns a reference to the property's default value if the map does not
    /// contain an entry for it.
    pub fn get_ref<P: Property>(&self, key: P) -> &P::Value {
        self.get_direct(key).unwrap_or_else(|| P::default_ref())
    }

    /// Get a reference to a style directly in this map (no default value).
    pub fn get_direct<P: Property>(&self, _: P) -> Option<&P::Value> {
        self.map
            .iter()
            .find(|pair| pair.0 == StyleId::of::<P>())
            .and_then(|pair| pair.1.downcast())
    }

    /// Apply styles from `outer` in-place.
    ///
    /// Properties from `self` take precedence over the ones from `outer`.
    pub fn apply(&mut self, outer: &Self) {
        'outer: for pair in &outer.map {
            for (id, entry) in &mut self.map {
                if pair.0 == *id {
                    entry.apply(&pair.1);
                    continue 'outer;
                }
            }

            self.map.push(pair.clone());
        }
    }

    /// Create new styles combining `self` with `outer`.
    ///
    /// Properties from `self` take precedence over the ones from `outer`.
    pub fn chain(&self, outer: &Self) -> Self {
        let mut styles = self.clone();
        styles.apply(outer);
        styles
    }

    /// Keep only those styles that are also in `other`.
    pub fn intersect(&mut self, other: &Self) {
        self.map.retain(|a| other.map.iter().any(|b| a == b));
    }

    /// Keep only those styles that are not also in `other`.
    pub fn erase(&mut self, other: &Self) {
        self.map.retain(|a| other.map.iter().all(|b| a != b));
    }

    /// Whether two style maps are equal when filtered down to the given
    /// properties.
    pub fn compatible<F>(&self, other: &Self, filter: F) -> bool
    where
        F: Fn(StyleId) -> bool,
    {
        // TODO(set): Filtered length + one direction equal should suffice.
        let f = |e: &&(StyleId, Entry)| filter(e.0);
        self.map.iter().filter(f).all(|pair| other.map.contains(pair))
            && other.map.iter().filter(f).all(|pair| self.map.contains(pair))
    }
}

impl Debug for Styles {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            for pair in &self.map {
                writeln!(f, "{:#?}", pair.1)?;
            }
            Ok(())
        } else {
            f.write_str("Styles ")?;
            f.debug_set().entries(self.map.iter().map(|pair| &pair.1)).finish()
        }
    }
}

impl PartialEq for Styles {
    fn eq(&self, other: &Self) -> bool {
        self.compatible(other, |_| true)
    }
}

/// An entry for a single style property.
#[derive(Clone)]
pub(crate) struct Entry(Rc<dyn Bounds>);

impl Entry {
    fn new<P: Property>(key: P, value: P::Value) -> Self {
        Self(Rc::new((key, value)))
    }

    fn downcast<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref()
    }

    fn apply(&mut self, outer: &Self) {
        *self = self.0.combine(outer);
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other)
    }
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash64());
    }
}

trait Bounds: 'static {
    fn as_any(&self) -> &dyn Any;
    fn fmt(&self, f: &mut Formatter) -> fmt::Result;
    fn dyn_eq(&self, other: &Entry) -> bool;
    fn hash64(&self) -> u64;
    fn combine(&self, outer: &Entry) -> Entry;
}

impl<P> Bounds for (P, P::Value)
where
    P: Property,
    P::Value: Debug + Hash + PartialEq + 'static,
{
    fn as_any(&self) -> &dyn Any {
        &self.1
    }

    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "#[{} = {:?}]", P::NAME, self.1)
        } else {
            write!(f, "{}: {:?}", P::NAME, self.1)
        }
    }

    fn dyn_eq(&self, other: &Entry) -> bool {
        if let Some(other) = other.downcast::<P::Value>() {
            &self.1 == other
        } else {
            false
        }
    }

    fn hash64(&self) -> u64 {
        // No need to hash the TypeId since there's only one
        // valid value type per property.
        fxhash::hash64(&self.1)
    }

    fn combine(&self, outer: &Entry) -> Entry {
        let outer = outer.downcast::<P::Value>().unwrap();
        let combined = P::combine(self.1.clone(), outer.clone());
        Entry::new(self.0, combined)
    }
}

/// Style property keys.
///
/// This trait is not intended to be implemented manually, but rather through
/// the `properties!` macro.
pub trait Property: Copy + 'static {
    /// The type of this property, for example, this could be
    /// [`Length`](crate::geom::Length) for a `WIDTH` property.
    type Value: Debug + Clone + Hash + PartialEq + 'static;

    /// The name of the property, used for debug printing.
    const NAME: &'static str;

    /// Combine the property with an outer value.
    fn combine(inner: Self::Value, outer: Self::Value) -> Self::Value;

    /// The default value of the property.
    fn default() -> Self::Value;

    /// A static reference to the default value of the property.
    ///
    /// This is automatically implemented through lazy-initialization in the
    /// `properties!` macro. This way, expensive defaults don't need to be
    /// recreated all the time.
    fn default_ref() -> &'static Self::Value;
}

/// A unique identifier for a style property.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct StyleId(TypeId);

impl StyleId {
    /// The style id of the property.
    pub fn of<P: Property>() -> Self {
        Self(TypeId::of::<P>())
    }
}

/// Generate the property keys for a node.
macro_rules! properties {
    ($node:ty, $(
        $(#[doc = $doc:expr])*
        $(#[fold($combine:expr)])?
        $name:ident: $type:ty = $default:expr
    ),* $(,)?) => {
        // TODO(set): Fix possible name clash.
        mod properties {
            use std::marker::PhantomData;
            use $crate::eval::{Property, StyleId};
            use super::*;

            $(#[allow(non_snake_case)] mod $name {
                use once_cell::sync::Lazy;
                use super::*;

                pub struct Key<T>(pub PhantomData<T>);

                impl<T> Copy for Key<T> {}
                impl<T> Clone for Key<T> {
                    fn clone(&self) -> Self {
                        *self
                    }
                }

                impl Property for Key<$type> {
                    type Value = $type;

                    const NAME: &'static str = concat!(
                        stringify!($node), "::", stringify!($name)
                    );

                    #[allow(unused_mut, unused_variables)]
                    fn combine(mut inner: Self::Value, outer: Self::Value) -> Self::Value {
                        $(
                            let combine: fn(Self::Value, Self::Value) -> Self::Value = $combine;
                            inner = combine(inner, outer);
                        )?
                        inner
                    }

                    fn default() -> Self::Value {
                        $default
                    }

                    fn default_ref() -> &'static Self::Value {
                        static LAZY: Lazy<$type> = Lazy::new(|| $default);
                        &*LAZY
                    }
                }
            })*

            impl $node {
                /// Check whether the property with the given type id belongs to
                /// `Self`.
                pub fn has_property(id: StyleId) -> bool {
                    false || $(id == StyleId::of::<$name::Key<$type>>())||*
                }

                $($(#[doc = $doc])* pub const $name: $name::Key<$type>
                    = $name::Key(PhantomData);)*
            }
        }
    };
}

/// Set a style property to a value if the value is `Some`.
macro_rules! set {
    ($styles:expr, $target:expr => $value:expr) => {
        if let Some(v) = $value {
            $styles.set($target, v);
        }
    };
}
