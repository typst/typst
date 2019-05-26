//! Error handling.

/// Create an error type.
macro_rules! error_type {
    (   // The variable used instead of self in functions
        // followed by the error type things are happening on.
        $var:ident: $err:ident,
        // Optionally the name of a result type to generate.
        $(res: $res:ident,)*
        // A `Display` and `Debug` implementation.
        show: $f:ident => $show:expr,
        // Optionally a `source` function for the `std::error::Error` trait.
        $(source: $source:expr,)*
        // Any number of `From` implementations.
        $(from: ($from:path, $conv:expr),)*
    ) => {
        // Possibly create a result type.
        $(type $res<T> = std::result::Result<T, $err>;)*

        impl std::fmt::Display for $err {
            fn fmt(&self, $f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let $var = self;
                $show
            }
        }

        impl std::fmt::Debug for $err {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Display::fmt(self, f)
            }
        }

        impl std::error::Error for $err {
            // The source method is only generated if an implementation was given.
            $(fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
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
