//! Trait and prelude for custom functions.

use crate::{Pass, Feedback};
use crate::syntax::parsing::{FuncCall, ParseState};
use crate::syntax::span::Span;

/// Types that are useful for creating your own functions.
pub mod prelude {
    pub use crate::{function, body, error, warning};
    pub use crate::layout::prelude::*;
    pub use crate::layout::Command::{self, *};
    pub use crate::style::{LayoutStyle, PageStyle, TextStyle};
    pub use crate::syntax::expr::*;
    pub use crate::syntax::model::SyntaxModel;
    pub use crate::syntax::span::{Span, Spanned};
    pub use crate::syntax::value::*;
    pub use super::OptionExt;
}

/// Parse a function from source code.
pub trait ParseFunc {
    /// A metadata type whose value is passed into the function parser. This
    /// allows a single function to do different things depending on the value
    /// that needs to be given when inserting the function into a
    /// [scope](crate::syntax::Scope).
    ///
    /// For example, the functions `word.spacing`, `line.spacing` and
    /// `par.spacing` are actually all the same function
    /// [`ContentSpacingFunc`](crate::library::ContentSpacingFunc) with the
    /// metadata specifiy which content should be spaced.
    type Meta: Clone;

    /// Parse the header and body into this function given a context.
    fn parse(
        header: FuncCall,
        state: &ParseState,
        metadata: Self::Meta,
    ) -> Pass<Self> where Self: Sized;
}

/// Extra methods on [`Options`](Option) used for argument parsing.
pub trait OptionExt<T>: Sized {
    /// Calls `f` with `val` if this is `Some(val)`.
    fn with(self, f: impl FnOnce(T));

    /// Reports an error about a missing argument with the given name and span
    /// if the option is `None`.
    fn or_missing(self, span: Span, arg: &str, f: &mut Feedback) -> Self;
}

impl<T> OptionExt<T> for Option<T> {
    fn with(self, f: impl FnOnce(T)) {
        if let Some(val) = self {
            f(val);
        }
    }

    fn or_missing(self, span: Span, arg: &str, f: &mut Feedback) -> Self {
        if self.is_none() {
            error!(@f, span, "missing argument: {}", arg);
        }
        self
    }
}

/// Allows to implement a function type concisely.
///
/// # Examples
/// Look at the source code of the [`library`](crate::library) module for more
/// examples on how the macro works.
#[macro_export]
macro_rules! function {
    // Entry point.
    ($(#[$outer:meta])* $v:vis $storage:ident $name:ident $($r:tt)*) => {
        function!(@def($name) $(#[$outer])* $v $storage $name $($r)*);
    };
    (@def($name:ident) $definition:item $($r:tt)*) => {
        $definition
        function!(@meta($name) $($r)*);
    };

    // Metadata.
    (@meta($name:ident) type Meta = $meta:ty; $($r:tt)*) => {
        function!(@parse($name, $meta) $($r)*);
    };
    (@meta($name:ident) $($r:tt)*) => {
        function!(@parse($name, ()) $($r)*);
    };

    // Parse trait.
    (@parse($($a:tt)*) parse(default) $($r:tt)*) => {
        function!(@parse($($a)*) parse(_h, _b, _c, _f, _m) {Default::default() } $($r)*);
    };
    (@parse($($a:tt)*) parse($h:ident, $b:ident, $c:ident, $f:ident) $($r:tt)* ) => {
        function!(@parse($($a)*) parse($h, $b, $c, $f, _metadata) $($r)*);
    };
    (@parse($name:ident, $meta:ty) parse(
        $header:ident,
        $body:ident,
        $state:ident,
        $feedback:ident,
        $metadata:ident
    ) $code:block $($r:tt)*) => {
        impl $crate::func::ParseFunc for $name {
            type Meta = $meta;

            fn parse(
                #[allow(unused)] mut call: $crate::syntax::parsing::FuncCall,
                #[allow(unused)] $state: &$crate::syntax::parsing::ParseState,
                #[allow(unused)] $metadata: Self::Meta,
            ) -> $crate::Pass<Self> where Self: Sized {
                let mut feedback = $crate::Feedback::new();
                #[allow(unused)] let $header = &mut call.header;
                #[allow(unused)] let $body = &mut call.body;
                #[allow(unused)] let $feedback = &mut feedback;

                let func = $code;

                for arg in call.header.args.pos.0 {
                    error!(@feedback, arg.span, "unexpected argument");
                }

                for arg in call.header.args.key.0 {
                    error!(@feedback, arg.span, "unexpected argument");
                }

                $crate::Pass::new(func, feedback)
            }
        }

        function!(@layout($name) $($r)*);
    };

    (@layout($name:ident) layout($this:ident, $ctx:ident, $feedback:ident) $code:block) => {
        impl $crate::syntax::model::Model for $name {
            fn layout<'a, 'b, 't>(
                #[allow(unused)] &'a $this,
                #[allow(unused)] mut $ctx: $crate::layout::LayoutContext<'b>,
            ) -> $crate::layout::DynFuture<'t, $crate::Pass<$crate::layout::Commands<'a>>>
            where
                'a: 't,
                'b: 't,
                Self: 't,
            {
                Box::pin(async move {
                    let mut feedback = $crate::Feedback::new();
                    #[allow(unused)] let $feedback = &mut feedback;
                    let commands = $code;
                    $crate::Pass::new(commands, feedback)
                })
            }
        }
    };
}

/// Parse the body of a function.
///
/// - If the function does not expect a body, use `body!(nope: body, feedback)`.
/// - If the function can have a body, use `body!(opt: body, state, feedback,
///   decos)`.
///
/// # Arguments
/// - The `$body` should be of type `Option<Spanned<&str>>`.
/// - The `$state` is the parse state to use.
/// - The `$feedback` should be a mutable references to a
///   [`Feedback`](crate::Feedback) struct which is filled with the feedback
///   information arising from parsing.
#[macro_export]
macro_rules! body {
    (opt: $body:expr, $state:expr, $feedback:expr) => ({
        $body.map(|body| {
            let parsed = $crate::syntax::parsing::parse(body.v, body.span.start, $state);
            $feedback.extend(parsed.feedback);
            parsed.output
        })
    });

    (nope: $body:expr, $feedback:expr) => {
        if let Some(body) = $body {
            error!(@$feedback, body.span, "unexpected body");
        }
    };
}
