//! Deduplicating maps and keys for argument parsing.

use std::collections::HashMap;
use std::hash::Hash;

use crate::func::prelude::*;


macro_rules! key {
    ($type:ty, $name:expr, $($patterns:tt)*) => {
        impl $type {
            /// Parse this key from an identifier.
            pub fn from_ident(ident: &Spanned<Ident>) -> ParseResult<Self> {
                Ok(match ident.v.as_str() {
                    $($patterns)*
                    _ => error!("expected {}", <Self as ExpressionKind>::NAME),
                })
            }
        }

        impl ExpressionKind for $type {
            const NAME: &'static str = $name;

            fn from_expr(expr: Spanned<Expr>) -> ParseResult<Self> {
                if let Expr::Ident(ident) = expr.v {
                    Self::from_ident(&Spanned::new(ident, expr.span))
                } else {
                    error!("expected {}", Self::NAME);
                }
            }
        }
    };
}

pub_use_mod!(axis);
pub_use_mod!(alignment);
pub_use_mod!(padding);


#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DefaultKey<T> {
    Some(T),
    None,
}

impl<T> Into<Option<T>> for DefaultKey<T> {
    fn into(self) -> Option<T> {
        match self {
            DefaultKey::Some(v) => Some(v),
            DefaultKey::None => None,
        }
    }
}

impl<T> ExpressionKind for DefaultKey<T> where T: ExpressionKind {
    const NAME: &'static str = T::NAME;

    fn from_expr(expr: Spanned<Expr>) -> ParseResult<DefaultKey<T>> {
        if let Expr::Ident(ident) = &expr.v {
            match ident.as_str() {
                "default" => return Ok(DefaultKey::None),
                _ => {},
            }
        }

        T::from_expr(expr).map(|v| DefaultKey::Some(v))
    }
}

/// A deduplicating map type useful for storing possibly redundant arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsistentMap<K, V> where K: Hash + Eq {
    map: HashMap<K, V>,
}

impl<K, V> ConsistentMap<K, V> where K: Hash + Eq {
    pub fn new() -> ConsistentMap<K, V> {
        ConsistentMap { map: HashMap::new() }
    }

    /// Add a key-value pair.
    pub fn add(&mut self, key: K, value: V) -> ParseResult<()> {
        match self.map.insert(key, value) {
            Some(_) => error!("duplicate argument"),
            None => Ok(())
        }
    }

    /// Add a key-value pair if the value is not `None`.
    pub fn add_opt(&mut self, key: K, value: Option<V>) -> ParseResult<()> {
        Ok(if let Some(value) = value {
            self.add(key, value)?;
        })
    }

    /// Get the value at a key if it is present.
    pub fn get(&self, key: K) -> Option<&V> {
        self.map.get(&key)
    }

    /// Call a function with the value if the key is present.
    pub fn with<F>(&self, key: K, callback: F) where F: FnOnce(&V) {
        if let Some(value) = self.map.get(&key) {
            callback(value);
        }
    }

    /// Create a new consistent map where keys and values are mapped to new keys
    /// and values.
    ///
    /// Returns an error if a new key is duplicate.
    pub fn dedup<F, K2, V2>(&self, f: F) -> LayoutResult<ConsistentMap<K2, V2>>
    where F: Fn(&K, &V) -> ParseResult<(K2, V2)>, K2: Hash + Eq {
        let mut map = ConsistentMap::new();

        for (key, value) in self.map.iter() {
            let (key, value) = f(key, value)?;
            map.add(key, value)?;
        }

        Ok(map)
    }

    /// Iterate over the (key, value) pairs.
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.map.iter()
    }
}
