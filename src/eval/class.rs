use std::fmt::{self, Debug, Formatter, Write};
use std::marker::PhantomData;
use std::rc::Rc;

use super::{Args, EvalContext, Node, Styles};
use crate::diag::TypResult;
use crate::util::EcoString;

/// A class of nodes.
#[derive(Clone)]
pub struct Class(Rc<Inner<dyn Bounds>>);

/// The unsized structure behind the [`Rc`].
struct Inner<T: ?Sized> {
    name: EcoString,
    dispatch: T,
}

impl Class {
    /// Create a new class.
    pub fn new<T>(name: EcoString) -> Self
    where
        T: Construct + Set + 'static,
    {
        Self(Rc::new(Inner {
            name,
            dispatch: Dispatch::<T>(PhantomData),
        }))
    }

    /// The name of the class.
    pub fn name(&self) -> &EcoString {
        &self.0.name
    }

    /// Construct an instance of the class.
    pub fn construct(&self, ctx: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        self.0.dispatch.construct(ctx, args)
    }

    /// Execute the class's set rule.
    pub fn set(&self, styles: &mut Styles, args: &mut Args) -> TypResult<()> {
        self.0.dispatch.set(styles, args)
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
        // We cast to thin pointers for comparison.
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
    fn set(styles: &mut Styles, args: &mut Args) -> TypResult<()>;
}

/// Zero-sized struct whose vtable contains the constructor and set rule of a
/// class.
struct Dispatch<T>(PhantomData<T>);

trait Bounds {
    fn construct(&self, ctx: &mut EvalContext, args: &mut Args) -> TypResult<Node>;
    fn set(&self, styles: &mut Styles, args: &mut Args) -> TypResult<()>;
}

impl<T> Bounds for Dispatch<T>
where
    T: Construct + Set,
{
    fn construct(&self, ctx: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        T::construct(ctx, args)
    }

    fn set(&self, styles: &mut Styles, args: &mut Args) -> TypResult<()> {
        T::set(styles, args)
    }
}
