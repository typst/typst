//! Auxiliary macros.


/// Create trait implementations for an error type.
macro_rules! error_type {
    (
        $this:ident: $type:ident,
        $(res: $res:ident,)*
        show: $f:ident => $show:expr,
        $(source: $source:expr,)*
        $(from: ($err:ident: $from:path, $conv:expr),)*
    ) => {
        // Possibly create a result type.
        $(type $res<T> = std::result::Result<T, $type>;)*

        impl std::fmt::Display for $type {
            fn fmt(&$this, $f: &mut std::fmt::Formatter) -> std::fmt::Result {
                $show
            }
        }

        debug_display!($type);

        impl std::error::Error for $type {
            // The source method is only generated if an implementation was given.
            $(fn source(&$this) -> Option<&(dyn std::error::Error + 'static)> {
                $source
            })*
        }

        // Create any number of from implementations.
        $(impl From<$from> for $type {
            fn from($err: $from) -> $type {
                $conv
            }
        })*
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

/// Whether an expression matches a set of patterns.
macro_rules! matches {
    ($expression:expr, $( $pattern:pat )|+ $( if $guard: expr )?) => {
        match $expression {
            $( $pattern )|+ $( if $guard )? => true,
            _ => false
        }
    }
}
