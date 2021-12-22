use std::fmt::{self, Debug, Formatter, Write};
use std::marker::PhantomData;
use std::rc::Rc;

use super::{Args, EvalContext, Node, Styles};
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
pub struct Class(Rc<Inner<dyn Bounds>>);

/// The unsized structure behind the [`Rc`].
struct Inner<T: ?Sized> {
    name: EcoString,
    shim: T,
}

impl Class {
    /// Create a new class.
    pub fn new<T>(name: EcoString) -> Self
    where
        T: Construct + Set + 'static,
    {
        // By specializing the shim to `T`, its vtable will contain T's
        // `Construct` and `Set` impls (through the `Bounds` trait), enabling us
        // to use them in the class's methods.
        Self(Rc::new(Inner { name, shim: Shim::<T>(PhantomData) }))
    }

    /// The name of the class.
    pub fn name(&self) -> &EcoString {
        &self.0.name
    }

    /// Construct an instance of the class.
    ///
    /// This parses both property and data arguments (in this order) and styles
    /// the node constructed from the data with the style properties.
    pub fn construct(&self, ctx: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        let mut styles = Styles::new();
        self.set(args, &mut styles)?;
        let node = self.0.shim.construct(ctx, args)?;
        Ok(node.styled(styles))
    }

    /// Execute the class's set rule.
    ///
    /// This parses property arguments and writes the resulting styles into the
    /// given style map. There are no further side effects.
    pub fn set(&self, args: &mut Args, styles: &mut Styles) -> TypResult<()> {
        self.0.shim.set(args, styles)
    }
}

impl Debug for Class {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("<class ")?;
        f.write_str(&self.0.name)?;
        f.write_char('>')
    }
}

impl PartialEq for Class {
    fn eq(&self, other: &Self) -> bool {
        // We cast to thin pointers for comparison because we don't want to
        // compare vtables (there can be duplicate vtables across codegen units).
        std::ptr::eq(
            Rc::as_ptr(&self.0) as *const (),
            Rc::as_ptr(&other.0) as *const (),
        )
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
    fn set(args: &mut Args, styles: &mut Styles) -> TypResult<()>;
}

/// Rewires the operations available on a class in an object-safe way. This is
/// only implemented by the zero-sized `Shim` struct.
trait Bounds {
    fn construct(&self, ctx: &mut EvalContext, args: &mut Args) -> TypResult<Node>;
    fn set(&self, args: &mut Args, styles: &mut Styles) -> TypResult<()>;
}

struct Shim<T>(PhantomData<T>);

impl<T> Bounds for Shim<T>
where
    T: Construct + Set,
{
    fn construct(&self, ctx: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        T::construct(ctx, args)
    }

    fn set(&self, args: &mut Args, styles: &mut Styles) -> TypResult<()> {
        T::set(args, styles)
    }
}
