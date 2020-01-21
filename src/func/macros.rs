//! Helper types and macros for creating custom functions.


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
        function!(@parse($($a)*) parse(_h, _b, _c, _e, _d, _m) {Default::default() } $($r)*);
    };
    (@parse($($a:tt)*) parse($h:ident, $b:ident, $c:ident, $e:ident, $d:ident) $($r:tt)* ) => {
        function!(@parse($($a)*) parse($h, $b, $c, $e, $d, _metadata) $($r)*);
    };
    (@parse($name:ident, $meta:ty) parse(
        $header:ident,
        $body:ident,
        $ctx:ident,
        $errors:ident,
        $decos:ident,
        $metadata:ident
    ) $code:block $($r:tt)*) => {
        impl $crate::func::Parse for $name {
            type Meta = $meta;

            fn parse(
                #[allow(unused)] mut $header: FuncHeader,
                #[allow(unused)] $body: Option<(Position, &str)>,
                #[allow(unused)] $ctx: ParseContext,
                #[allow(unused)] $metadata: Self::Meta,
            ) -> Parsed<Self> where Self: Sized {
                #[allow(unused)] let mut $errors = vec![];
                #[allow(unused)] let mut $decos = vec![];
                let output = $code;
                $crate::syntax::Parsed { output, errors: $errors, decorations: $decos }
            }
        }

        function!(@layout($name) $($r)*);
    };

    (@layout($name:ident) layout($this:ident, $ctx:ident, $errors:ident) $code:block) => {
        impl $crate::syntax::Model for $name {
            fn layout<'a, 'b, 'c, 't>(
                #[allow(unused)] &'a $this,
                #[allow(unused)] $ctx: $crate::layout::LayoutContext<'b, 'c>,
            ) -> $crate::syntax::DynFuture<'t, $crate::layout::Layouted<$crate::func::Commands<'a>>>
            where
                'a: 't,
                'b: 't,
                'c: 't,
                Self: 't,
            {
                Box::pin(async move {
                    #[allow(unused)] let mut $errors = vec![];
                    let output = $code;
                    $crate::layout::Layouted { output, errors: $errors }
                })
            }
        }
    };
}

/// Parse the body of a function.
///
/// - If the function does not expect a body, use `parse!(forbidden: body)`.
/// - If the function can have a body, use `parse!(optional: body, ctx)`.
/// - If the function must have a body, use `parse!(expected: body, ctx)`.
#[macro_export]
macro_rules! body {
    (opt: $body:expr, $ctx:expr, $errors:expr, $decos:expr) => ({
        $body.map(|body| {
            let parsed = $crate::syntax::parse(body.0, body.1, $ctx);
            $errors.extend(parsed.errors);
            $decos.extend(parsed.decorations);
            parsed.output
        })
    });

    (nope: $body:expr, $errors:expr) => {
        if let Some(body) = $body {
            $errors.push($crate::err!(body.span, "unexpected body"));
        }
    };
}

/// Construct an error with an optional span.
#[macro_export]
macro_rules! err {
    (@$severity:ident: $span:expr; $($args:tt)*) => {
        $crate::syntax::Spanned { v: err!(@Error: $($args)*), span: $span }
    };

    (@$severity:ident: $($args:tt)*) => {
        $crate::error::Error {
            message: format!($($args)*),
            severity: $crate::error::Severity::$severity,
        }
    };

    ($($tts:tt)*) => { err!(@Error: $($tts)*) };
}
