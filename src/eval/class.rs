use std::fmt::{self, Debug, Formatter, Write};

use super::{Args, EvalContext, Node, StyleMap};
use crate::diag::TypResult;
use crate::util::EcoString;

/// A class of [nodes](Node).
///
/// You can [construct] an instance of a class in Typst code by invoking the
/// class as a callable. This always produces some node, but not necessarily one
/// of fixed type. For example, the `text` constructor does not actually create
/// a [`TextNode`]. Instead it applies styling to whatever node you pass in and
/// returns it structurally unchanged.
///
/// The arguments you can pass to a class constructor fall into two categories:
/// Data that is inherent to the instance (e.g. the text of a heading) and style
/// properties (e.g. the fill color of a heading). As the latter are often
/// shared by many instances throughout a document, they can also be
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
    name: EcoString,
    construct: fn(&mut EvalContext, &mut Args) -> TypResult<Node>,
    set: fn(&mut Args, &mut StyleMap) -> TypResult<()>,
}

impl Class {
    /// Create a new class.
    pub fn new<T>(name: EcoString) -> Self
    where
        T: Construct + Set + 'static,
    {
        Self {
            name,
            construct: T::construct,
            set: T::set,
        }
    }

    /// The name of the class.
    pub fn name(&self) -> &EcoString {
        &self.name
    }

    /// Construct an instance of the class.
    ///
    /// This parses both property and data arguments (in this order) and styles
    /// the node constructed from the data with the style properties.
    pub fn construct(&self, ctx: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        let mut styles = StyleMap::new();
        self.set(args, &mut styles)?;
        let node = (self.construct)(ctx, args)?;
        Ok(node.styled_with_map(styles.scoped()))
    }

    /// Execute the class's set rule.
    ///
    /// This parses property arguments and writes the resulting styles into the
    /// given style map. There are no further side effects.
    pub fn set(&self, args: &mut Args, styles: &mut StyleMap) -> TypResult<()> {
        (self.set)(args, styles)
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

/// Construct an instance of a class.
pub trait Construct {
    /// Construct an instance of this class from the arguments.
    ///
    /// This is passed only the arguments that remain after execution of the
    /// class's set rule.
    fn construct(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Node>;
}

/// Set style properties of a class.
pub trait Set {
    /// Parse the arguments and insert style properties of this class into the
    /// given style map.
    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()>;
}
