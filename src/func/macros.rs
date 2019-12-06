//! Helper types and macros for creating custom functions.

/// Defines function types concisely.
#[macro_export]
macro_rules! function {
    // Parse a unit struct.
    ($(#[$outer:meta])* pub struct $type:ident; $($rest:tt)*) => {
        $(#[$outer])*
        pub struct $type;
        function!(@meta $type | $($rest)*);
    };

    // Parse a tuple struct.
    ($(#[$outer:meta])* pub struct $type:ident($($fields:tt)*); $($rest:tt)*) => {
        $(#[$outer])*
        pub struct $type($($fields)*);
        function!(@meta $type | $($rest)*);
    };

    // Parse a struct with fields.
    ($(#[$outer:meta])* pub struct $type:ident { $($fields:tt)* } $($rest:tt)*) => {
        $(#[$outer])*
        pub struct $type { $($fields)* }
        function!(@meta $type | $($rest)*);
    };

    // Parse a metadata type definition.
    (@meta $type:ident | type Meta = $meta:ty; $($rest:tt)*) => {
        function!(@parse $type $meta | $($rest)*);
    };

    // Set the metadata to `()` if there is no type definition.
    (@meta $type:ident | $($rest:tt)*) => {
        function!(@parse $type () | $($rest)*);
    };

    // Parse a `parse(default)`.
    (@parse $type:ident $meta:ty | parse(default) $($rest:tt)*) => {
        function!(@parse $type $meta |
            parse(_args, _body, _ctx, _meta) { Default::default() }
            $($rest)*
        );
    };

    // (0-arg) Parse a parse-definition without arguments.
    (@parse $type:ident $meta:ty | parse() $code:block $($rest:tt)*) => {
        function!(@parse $type $meta | parse(_args, _body, _ctx, _meta) $code $($rest)*);
    };

    // (1-arg) Parse a parse-definition with only the first argument.
    (@parse $type:ident $meta:ty | parse($args:ident) $code:block $($rest:tt)*) => {
        function!(@parse $type $meta | parse($args, _body, _ctx, _meta) $code $($rest)*);
    };

    // (2-arg) Parse a parse-definition with only the first two arguments.
    (@parse $type:ident $meta:ty |
        parse($args:ident, $body:pat) $code:block $($rest:tt)*
    ) => {
        function!(@parse $type $meta | parse($args, $body, _ctx, _meta) $code $($rest)*);
    };

    // (3-arg) Parse a parse-definition with only the first three arguments.
    (@parse $type:ident $meta:ty |
        parse($args:ident, $body:pat, $ctx:pat) $code:block $($rest:tt)*
    ) => {
        function!(@parse $type $meta | parse($args, $body, $ctx, _meta) $code $($rest)*);
    };

    // (4-arg) Parse a parse-definition with all four arguments.
    (@parse $type:ident $meta:ty |
        parse($args:ident, $body:pat, $ctx:pat, $metadata:pat) $code:block
        $($rest:tt)*
    ) => {
        impl $crate::func::ParseFunc for $type {
            type Meta = $meta;

            fn parse(
                args: FuncArgs,
                $body: Option<Spanned<&str>>,
                $ctx: ParseContext,
                $metadata: Self::Meta,
            ) -> ParseResult<Self> where Self: Sized {
                #[allow(unused_mut)]
                let mut $args = args;
                let val = $code;
                if !$args.is_empty() {
                    error!(unexpected_argument);
                }
                Ok(val)
            }
        }

        function!(@layout $type | $($rest)*);
    };

    // (0-arg) Parse a layout-definition without arguments.
    (@layout $type:ident | layout() $code:block) => {
        function!(@layout $type | layout(self, _ctx) $code);
    };

    // (1-arg) Parse a layout-definition with only the first argument.
    (@layout $type:ident | layout($this:ident) $code:block) => {
        function!(@layout $type | layout($this, _ctx) $code);
    };

    // (2-arg) Parse a layout-definition with all arguments.
    (@layout $type:ident | layout($this:ident, $ctx:pat) $code:block) => {
        impl $crate::func::LayoutFunc for $type {
            fn layout(&$this, $ctx: LayoutContext) -> LayoutResult<Commands> {
                Ok($code)
            }
        }
    };
}

/// Parse the body of a function.
/// - If the function does not expect a body, use `parse!(forbidden: body)`.
/// - If the function can have a body, use `parse!(optional: body, ctx)`.
/// - If the function must have a body, use `parse!(expected: body, ctx)`.
#[macro_export]
macro_rules! parse {
    (forbidden: $body:expr) => {
        if $body.is_some() {
            error!("unexpected body");
        }
    };

    (optional: $body:expr, $ctx:expr) => (
        if let Some(body) = $body {
            Some($crate::syntax::parse(body.v, $ctx)?)
        } else {
            None
        }
    );

    (expected: $body:expr, $ctx:expr) => (
        if let Some(body) = $body {
            $crate::syntax::parse(body.v, $ctx)?
        } else {
            error!("expected body");
        }
    )
}

/// Early-return with a formatted typesetting error or construct an error
/// expression without returning when prefixed with `@`.
#[macro_export]
macro_rules! error {
    (@unexpected_argument) => (error!(@"unexpected argument"));
    (@$($tts:tt)*) => ($crate::TypesetError::with_message(format!($($tts)*)));
    ($($tts:tt)*) => (return Err(error!(@$($tts)*)););
}
