use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

// Possible optimizations:
// - Ref-count map for cheaper cloning and smaller footprint
// - Store map in `Option` to make empty maps non-allocating
// - Store small properties inline

/// A map of style properties.
#[derive(Default, Clone)]
pub struct Styles {
    map: HashMap<TypeId, Rc<dyn Any>>,
}

impl Styles {
    /// Create a new, empty style map.
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    /// Set the value for a style property.
    pub fn set<P: Property>(&mut self, _: P, value: P::Value) {
        self.map.insert(TypeId::of::<P>(), Rc::new(value));
    }

    /// Get the value of a copyable style property.
    ///
    /// Returns the property's default value if the map does not contain an
    /// entry for it.
    pub fn get<P: Property>(&self, key: P) -> P::Value
    where
        P::Value: Copy,
    {
        self.get_inner(key).copied().unwrap_or_else(P::default)
    }

    /// Get a reference to a style property.
    ///
    /// Returns a reference to the property's default value if the map does not
    /// contain an entry for it.
    pub fn get_ref<P: Property>(&self, key: P) -> &P::Value {
        self.get_inner(key).unwrap_or_else(|| P::default_ref())
    }

    /// Get a reference to a style directly in this map.
    fn get_inner<P: Property>(&self, _: P) -> Option<&P::Value> {
        self.map
            .get(&TypeId::of::<P>())
            .and_then(|boxed| boxed.downcast_ref())
    }
}

impl Debug for Styles {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // TODO(set): Better debug printing possible?
        f.pad("Styles(..)")
    }
}

/// Stylistic property keys.
pub trait Property: 'static {
    /// The type of this property, for example, this could be
    /// [`Length`](crate::geom::Length) for a `WIDTH` property.
    type Value;

    /// The default value of the property.
    fn default() -> Self::Value;

    /// A static reference to the default value of the property.
    ///
    /// This is automatically implemented through lazy-initialization in the
    /// `properties!` macro. This way, expensive defaults don't need to be
    /// recreated all the time.
    fn default_ref() -> &'static Self::Value;
}

macro_rules! set {
    ($ctx:expr, $target:expr => $source:expr) => {
        if let Some(v) = $source {
            $ctx.styles.set($target, v);
        }
    };
}

macro_rules! properties {
    ($node:ty, $(
        $(#[$attr:meta])*
        $name:ident: $type:ty = $default:expr
    ),* $(,)?) => {
        // TODO(set): Fix possible name clash.
        mod properties {
            use std::marker::PhantomData;
            use super::*;

            $(#[allow(non_snake_case)] mod $name {
                use $crate::eval::Property;
                use once_cell::sync::Lazy;
                use super::*;

                pub struct Key<T>(pub PhantomData<T>);

                impl Property for Key<$type> {
                    type Value = $type;

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
                $($(#[$attr])* pub const $name: $name::Key<$type>
                    = $name::Key(PhantomData);)*
            }
        }
    };
}
