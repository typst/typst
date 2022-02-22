use std::any::TypeId;
use std::fmt::{self, Debug, Formatter, Write};
use std::hash::{Hash, Hasher};

use super::{Args, Func, StyleMap, Template, Value};
use crate::diag::TypResult;
use crate::Context;

/// A class of nodes.
///
/// You can [construct] an instance of a class in Typst code by invoking the
/// class as a callable. This always produces a template value, but not
/// necessarily a simple inline or block node. For example, the `text`
/// constructor does not actually create a [`TextNode`]. Instead it applies
/// styling to whatever node you pass in and returns it structurally unchanged.
///
/// The arguments you can pass to a class constructor fall into two categories:
/// Data that is inherent to the instance (e.g. the text/content of a heading)
/// and style properties (e.g. the fill color of a heading). As the latter are
/// often shared by many instances throughout a document, they can also be
/// conveniently configured through class's [`set`] rule. Then, they apply to
/// all nodes that are instantiated into the template where the `set` was
/// executed.
///
/// ```typst
/// This is normal.
/// [
///   #set text(weight: "bold")
///   #set heading(fill: blue)
///   = A blue & bold heading
/// ]
/// Normal again.
/// ```
///
/// [construct]: Self::construct
/// [`TextNode`]: crate::library::TextNode
/// [`set`]: Self::set
#[derive(Clone)]
pub struct Class {
    name: &'static str,
    id: TypeId,
    construct: fn(&mut Context, &mut Args) -> TypResult<Value>,
    set: fn(&mut Args, &mut StyleMap) -> TypResult<()>,
}

impl Class {
    /// Create a new class.
    pub fn new<T>(name: &'static str) -> Self
    where
        T: Construct + Set + 'static,
    {
        Self {
            name,
            id: TypeId::of::<T>(),
            construct: |ctx, args| {
                let mut styles = StyleMap::new();
                T::set(args, &mut styles)?;
                let template = T::construct(ctx, args)?;
                Ok(Value::Template(template.styled_with_map(styles.scoped())))
            },
            set: T::set,
        }
    }

    /// The name of the class.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The type id of the class.
    pub fn id(&self) -> TypeId {
        self.id
    }

    /// Return the class constructor as a function.
    pub fn constructor(&self) -> Func {
        Func::native(self.name, self.construct)
    }

    /// Construct an instance of the class.
    ///
    /// This parses both property and data arguments (in this order), styles the
    /// template constructed from the data with the style properties and wraps
    /// it in a value.
    pub fn construct(&self, ctx: &mut Context, mut args: Args) -> TypResult<Value> {
        let value = (self.construct)(ctx, &mut args)?;
        args.finish()?;
        Ok(value)
    }

    /// Execute the class's set rule.
    ///
    /// This parses property arguments and return the resulting styles.
    pub fn set(&self, mut args: Args) -> TypResult<StyleMap> {
        let mut styles = StyleMap::new();
        (self.set)(&mut args, &mut styles)?;
        args.finish()?;
        Ok(styles)
    }
}

impl Debug for Class {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("<class ")?;
        f.write_str(self.name())?;
        f.write_char('>')
    }
}

impl PartialEq for Class {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Class {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.construct as usize).hash(state);
        (self.set as usize).hash(state);
    }
}

/// Construct an instance of a class.
pub trait Construct {
    /// Construct an instance of this class from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// class's set rule.
    fn construct(ctx: &mut Context, args: &mut Args) -> TypResult<Template>;
}

/// Set style properties of a class.
pub trait Set {
    /// Parse the arguments and insert style properties of this class into the
    /// given style map.
    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()>;
}
