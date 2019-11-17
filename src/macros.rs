//! Auxiliary macros.

/// Create trait implementations for an error type.
macro_rules! error_type {
    (
        $var:ident: $err:ident,
        $(res: $res:ident,)*
        show: $f:ident => $show:expr,
        $(source: $source:expr,)*
        $(from: ($from:path, $conv:expr),)*
    ) => {
        // Possibly create a result type.
        $(type $res<T> = std::result::Result<T, $err>;)*

        impl std::fmt::Display for $err {
            fn fmt(&self, $f: &mut std::fmt::Formatter) -> std::fmt::Result {
                #[allow(unused)]
                let $var = self;
                $show
            }
        }

        debug_display!($err);

        impl std::error::Error for $err {
            // The source method is only generated if an implementation was given.
            $(fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                #[allow(unused)]
                let $var = self;
                $source
            })*
        }

        // Create any number of from implementations.
        $(impl From<$from> for $err {
            fn from($var: $from) -> $err {
                $conv
            }
        })*
    };
}

/// Shorthand for checking whether an expression matches a pattern.
macro_rules! matches {
    ($expr:expr, $($pattern:tt)*) => {
        match $expr {
            $($pattern)* => true,
            _ => false,
        }
    };
}

/// Create a `Debug` implementation from a `Display` implementation.
macro_rules! debug_display {
    ($type:ident) => (
        impl std::fmt::Debug for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Display::fmt(self, f)
            }
        }
    );
    ($type:ident; $generics:tt where $($bounds:tt)*) => (
        impl<$generics> std::fmt::Debug for $type<$generics> where $($bounds)* {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Display::fmt(self, f)
            }
        }
    );
}

/// Declare a module and reexport all its contents.
macro_rules! pub_use_mod {
    ($name:ident) => {
        mod $name;
        pub use $name::*;
    };
}
