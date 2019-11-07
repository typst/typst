//! Helper types and macros for creating custom functions.

use super::prelude::*;
use Expression::*;

/// Lets you implement the function trait more concisely.
#[macro_export]
macro_rules! function {
    (data: $ident:ident, $($tts:tt)*) => (
        #[allow(unused_imports)]
        use $crate::func::prelude::*;

        impl Function for $ident {
            function!(@parse $ident, $($tts)*);
        }
    );

    (@parse $ident:ident, parse: plain, $($tts:tt)*) => (
        fn parse(header: &FuncHeader, body: Option<&str>, _: ParseContext) -> ParseResult<Self>
        where Self: Sized {
            ArgParser::new(&header.args).done()?;
            if body.is_some() {
                err!("expected no body");
            }
            Ok($ident)
        }

        function!(@layout $($tts)*);
    );

    (
        @parse $ident:ident,
        parse($args:ident, $body:ident, $ctx:ident)
        $block:block
        $($tts:tt)*
    ) => (
        fn parse(header: &FuncHeader, body: Option<&str>, ctx: ParseContext) -> ParseResult<Self>
        where Self: Sized {
            #[allow(unused_mut)]
            let mut $args = ArgParser::new(&header.args);
            let $body = body;
            let $ctx = ctx;
            $block
        }

        function!(@layout $($tts)*);
    );

    (@layout layout($this:pat, $ctx:pat) $block:block) => (
        fn layout(&self, ctx: LayoutContext) -> LayoutResult<CommandList> {
            let $ctx = ctx;
            let $this = self;
            $block
        }
    );
}

/// Parse the body of a function.
/// - If the function does not expect a body, use `forbidden`.
/// - If the function can have a body, use `optional`.
/// - If the function must have a body, use `required`.
#[macro_export]
macro_rules! parse {
    (forbidden: $body:expr) => {
        if $body.is_some() {
            err!("unexpected body");
        }
    };

    (optional: $body:expr, $ctx:expr) => (
        if let Some(body) = $body {
            Some($crate::syntax::parse(body, $ctx)?)
        } else {
            None
        }
    );

    (required: $body:expr, $ctx:expr) => (
        if let Some(body) = $body {
            $crate::syntax::parse(body, $ctx)?
        } else {
            err!("expected body");
        }
    )
}

/// Early-return with a formatted parsing error or yield
/// an error expression without returning when prefixed with `@`.
#[macro_export]
macro_rules! err {
    (@$($tts:tt)*) => ($crate::syntax::ParseError::new(format!($($tts)*)));
    ($($tts:tt)*) => (return Err(err!(@$($tts)*)););
}

/// Easy parsing of function arguments.
pub struct ArgParser<'a> {
    args: &'a FuncArgs,
    positional_index: usize,
}

impl<'a> ArgParser<'a> {
    pub fn new(args: &'a FuncArgs) -> ArgParser<'a> {
        ArgParser {
            args,
            positional_index: 0,
        }
    }

    /// Get the next positional argument of the given type.
    ///
    /// If there are no more arguments or the type is wrong,
    /// this will return an error.
    pub fn get_pos<T>(&mut self) -> ParseResult<Spanned<T::Output>> where T: Argument<'a> {
        self.get_pos_opt::<T>()?
            .ok_or_else(|| err!(@"expected {}", T::ERROR_MESSAGE))
    }

    /// Get the next positional argument if there is any.
    ///
    /// If the argument is of the wrong type, this will return an error.
    pub fn get_pos_opt<T>(&mut self) -> ParseResult<Option<Spanned<T::Output>>>
    where T: Argument<'a> {
        let arg = self.args.positional
            .get(self.positional_index)
            .map(T::from_expr)
            .transpose();

        if let Ok(Some(_)) = arg {
            self.positional_index += 1;
        }

        arg
    }

    /// Get a keyword argument with the given key and type.
    pub fn get_key<T>(&mut self, key: &str) -> ParseResult<Spanned<T::Output>>
    where T: Argument<'a> {
        self.get_key_opt::<T>(key)?
            .ok_or_else(|| err!(@"expected {}", T::ERROR_MESSAGE))
    }

    /// Get a keyword argument with the given key and type if it is present.
    pub fn get_key_opt<T>(&mut self, key: &str) -> ParseResult<Option<Spanned<T::Output>>>
    where T: Argument<'a> {
        self.args.keyword.iter()
            .find(|entry| entry.val.0.val == key)
            .map(|entry| T::from_expr(&entry.val.1))
            .transpose()
    }

    /// Assert that there are no positional arguments left. Returns an error, otherwise.
    pub fn done(&self) -> ParseResult<()> {
        if self.positional_index == self.args.positional.len() {
            Ok(())
        } else {
            err!("unexpected argument");
        }
    }
}

/// A kind of argument.
pub trait Argument<'a> {
    type Output;
    const ERROR_MESSAGE: &'static str;

    fn from_expr(expr: &'a Spanned<Expression>) -> ParseResult<Spanned<Self::Output>>;
}

macro_rules! arg {
    ($type:ident, $err:expr, $doc:expr, $output:ty, $wanted:pat => $converted:expr) => (
        #[doc = $doc]
        #[doc = " argument for use with the [`ArgParser`]."]
        pub struct $type;
        impl<'a> Argument<'a> for $type {
            type Output = $output;
            const ERROR_MESSAGE: &'static str = $err;

            fn from_expr(expr: &'a Spanned<Expression>) -> ParseResult<Spanned<Self::Output>> {
                match &expr.val {
                    $wanted => Ok(Spanned::new($converted, expr.span)),
                    #[allow(unreachable_patterns)] _ => err!("expected {}", $err),
                }
            }
        }
    );
}

arg!(ArgExpr,  "expression", "A generic expression", &'a Expression, expr => &expr);
arg!(ArgIdent, "identifier", "An identifier (e.g. `horizontal`)", &'a str, Ident(s) => s.as_str());
arg!(ArgStr,   "string", "A string (e.g. `\"Hello\"`)", &'a str, Str(s) => s.as_str());
arg!(ArgNum,   "number", "A number (e.g. `5.4`)", f64, Num(n) => *n);
arg!(ArgSize,  "size", "A size (e.g. `12pt`)", crate::size::Size, Size(s) => *s);
arg!(ArgBool,  "bool", "A boolean (`true` or `false`)", bool, Bool(b) => *b);
