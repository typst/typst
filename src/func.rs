//! Tools for building custom functions.

use crate::Feedback;
use crate::syntax::span::{Span, Spanned};
use crate::syntax::parsing::{parse, ParseState};
use crate::syntax::tree::SyntaxTree;

/// Useful things for creating functions.
pub mod prelude {
    pub use crate::layout::prelude::*;
    pub use crate::layout::Command::{self, *};
    pub use crate::syntax::prelude::*;
    pub use crate::style::*;
    pub use super::{OptionExt, parse_maybe_body, expect_no_body};
}

/// Extra methods on [`Options`](Option) used for function argument parsing.
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

/// Parses a function's body if there is one or returns `None` otherwise.
pub fn parse_maybe_body(
    body: Option<Spanned<&str>>,
    state: &ParseState,
    f: &mut Feedback,
) -> Option<SyntaxTree> {
    body.map(|body| {
        let parsed = parse(body.v, body.span.start, state);
        f.extend(parsed.feedback);
        parsed.output
    })
}

/// Generates an error if there is function body even though none was expected.
pub fn expect_no_body(body: Option<Spanned<&str>>, f: &mut Feedback) {
    if let Some(body) = body {
        error!(@f, body.span, "unexpected body");
    }
}

/// Implement a custom function concisely.
///
/// # Examples
/// Look at the source code of the [`library`](crate::library) module for
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
        impl $crate::syntax::parsing::ParseCall for $name {
            type Meta = $meta;

            fn parse(
                #[allow(unused)] mut call: $crate::syntax::parsing::FuncCall,
                #[allow(unused)] $state: &$crate::syntax::parsing::ParseState,
                #[allow(unused)] $metadata: Self::Meta,
            ) -> $crate::Pass<Self> where Self: Sized {
                let mut feedback = $crate::Feedback::new();
                #[allow(unused)] let $header = &mut call.header;
                #[allow(unused)] let $body = call.body;
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

    (@layout($name:ident) layout(
        $this:ident,
        $ctx:ident,
        $feedback:ident
    ) $code:block) => {
        impl $crate::layout::Layout for $name {
            fn layout<'a, 'b, 't>(
                #[allow(unused)] &'a $this,
                #[allow(unused)] mut $ctx: $crate::layout::LayoutContext<'b>,
            ) -> $crate::DynFuture<'t, $crate::Pass<$crate::layout::Commands<'a>>>
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
