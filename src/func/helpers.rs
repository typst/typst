use std::iter::Peekable;
use std::slice::Iter;
use super::prelude::*;

/// Implement the function trait more concisely.
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
        fn parse(header: &FuncHeader, body: Option<&str>, _: ParseContext)
            -> ParseResult<Self> where Self: Sized
        {
            Arguments::new(header).done()?;
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
        fn parse(header: &FuncHeader, body: Option<&str>, ctx: ParseContext)
            -> ParseResult<Self> where Self: Sized
        {
            #[allow(unused_mut)] let mut $args = Arguments::new(header);
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

/// Return a formatted parsing error.
#[macro_export]
macro_rules! err {
    (@$($tts:tt)*) => ($crate::syntax::ParseError::new(format!($($tts)*)));
    ($($tts:tt)*) => (return Err(err!(@$($tts)*)););
}

/// Convenient interface for parsing function arguments.
pub struct Arguments<'a> {
    args: Peekable<Iter<'a, Spanned<Expression>>>,
}

impl<'a> Arguments<'a> {
    pub fn new(header: &'a FuncHeader) -> Arguments<'a> {
        Arguments {
            args: header.args.positional.iter().peekable()
        }
    }

    pub fn get_expr(&mut self) -> ParseResult<&'a Spanned<Expression>> {
        self.args.next()
            .ok_or_else(|| ParseError::new("expected expression"))
    }

    pub fn get_ident(&mut self) -> ParseResult<Spanned<&'a str>> {
        let expr = self.get_expr()?;
        match &expr.val {
            Expression::Ident(s) => Ok(Spanned::new(s.as_str(), expr.span)),
            _ => err!("expected identifier"),
        }
    }

    pub fn get_ident_if_present(&mut self) -> ParseResult<Option<Spanned<&'a str>>> {
        if self.args.peek().is_some() {
            self.get_ident().map(|s| Some(s))
        } else {
            Ok(None)
        }
    }

    pub fn done(&mut self) -> ParseResult<()> {
        if self.args.peek().is_none() {
            Ok(())
        } else {
            Err(ParseError::new("unexpected argument"))
        }
    }
}
