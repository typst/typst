/// Create an error type.
macro_rules! error_type {
    (
        $var:ident: $err:ident,
        $(res: $res:ident,)*
        show: $f:ident => $show:expr,
        $(source: $source:expr,)*
        $(from: ($from:path, $conv:expr),)*
    ) => {
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
            $(fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                let $var = self;
                $source
            })*
        }

        $(impl From<$from> for $err {
            fn from($var: $from) -> $err {
                $conv
            }
        })*
    };
}
