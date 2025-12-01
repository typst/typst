use std::sync::LazyLock;

use icu_properties::CanonicalCombiningClass;
use icu_properties::maps::CodePointMapData;
use icu_provider::AsDeserializingBufferProvider;
use icu_provider_blob::BlobDataProvider;

use crate::foundations::{Content, NativeElement, Str, SymbolElem, cast, elem, func};
use crate::layout::{Length, Rel};
use crate::math::Mathy;

/// Attaches an accent to a base.
///
/// # Example
/// ```example
/// $grave(a) = accent(a, `)$ \
/// $arrow(a) = accent(a, arrow)$ \
/// $tilde(a) = accent(a, \u{0303})$
/// ```
#[elem(Mathy)]
pub struct AccentElem {
    /// The base to which the accent is applied. May consist of multiple
    /// letters.
    ///
    /// ```example
    /// $arrow(A B C)$
    /// ```
    #[required]
    pub base: Content,

    /// The accent to apply to the base.
    ///
    /// Supported accents include:
    ///
    /// | Accent        | Name            | Codepoint |
    /// | ------------- | --------------- | --------- |
    /// | Grave         | `grave`         | <code>&DiacriticalGrave;</code> |
    /// | Acute         | `acute`         | `´`       |
    /// | Circumflex    | `hat`           | `^`       |
    /// | Tilde         | `tilde`         | `~`       |
    /// | Macron        | `macron`        | `¯`       |
    /// | Dash          | `dash`          | `‾`       |
    /// | Breve         | `breve`         | `˘`       |
    /// | Dot           | `dot`           | `.`       |
    /// | Double dot, Diaeresis | `dot.double`, `diaer` | `¨` |
    /// | Triple dot    | `dot.triple`    | <code>&tdot;</code> |
    /// | Quadruple dot | `dot.quad`      | <code>&DotDot;</code> |
    /// | Circle        | `circle`        | `∘`       |
    /// | Double acute  | `acute.double`  | `˝`       |
    /// | Caron         | `caron`         | `ˇ`       |
    /// | Right arrow   | `arrow`, `->`   | `→`       |
    /// | Left arrow    | `arrow.l`, `<-` | `←`       |
    /// | Left/Right arrow | `arrow.l.r`  | `↔`       |
    /// | Right harpoon | `harpoon`       | `⇀`       |
    /// | Left harpoon  | `harpoon.lt`    | `↼`       |
    #[required]
    pub accent: Accent,

    /// The size of the accent, relative to the width of the base.
    ///
    /// ```example
    /// $dash(A, size: #150%)$
    /// ```
    #[default(Rel::one())]
    pub size: Rel<Length>,

    /// Whether to remove the dot on top of lowercase i and j when adding a top
    /// accent.
    ///
    /// This enables the `dtls` OpenType feature.
    ///
    /// ```example
    /// $hat(dotless: #false, i)$
    /// ```
    #[default(true)]
    pub dotless: bool,
}

/// An accent character.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Accent(pub char);

impl Accent {
    /// Normalize a character into an accent.
    pub fn new(c: char) -> Self {
        Self(Self::combine(c).unwrap_or(c))
    }

    /// Tries to select the appropriate combining accent for a string, falling
    /// back to the string's lone character if there is no corresponding one.
    ///
    /// Returns `None` if there isn't one and the string has more than one
    /// character.
    pub fn normalize(s: &str) -> Option<Self> {
        Self::combining(s).or_else(|| s.parse::<char>().ok().map(Self))
    }

    /// Whether this accent is a bottom accent or not.
    pub fn is_bottom(&self) -> bool {
        static COMBINING_CLASS_DATA: LazyLock<CodePointMapData<CanonicalCombiningClass>> =
            LazyLock::new(|| {
                icu_properties::maps::load_canonical_combining_class(
                    &BlobDataProvider::try_new_from_static_blob(typst_assets::icu::ICU)
                        .unwrap()
                        .as_deserializing(),
                )
                .unwrap()
            });

        matches!(
            COMBINING_CLASS_DATA.as_borrowed().get(self.0),
            CanonicalCombiningClass::Below
        )
    }
}

/// This macro generates accent-related functions.
///
/// ```ignore
/// accents! {
///     '\u{0300}', "\u{0300}" | "`" => grave,
/// //  ^^^^^^^^^   ^^^^^^^^^^^^^^^^    ^^^^^
/// //  |           |                   |
/// //  |           |                   +-- The name of the function.
/// //  |           +--------- The list of strings that normalize to the accent.
/// //  +---------------------- The primary character that represents the accent.
/// }
/// ```
///
/// When combined with the `Accent::combine` function, accent characters can be normalized
/// to the primary character.
macro_rules! accents {
    ($($primary:literal, $($option:literal)|* => $name:ident),* $(,)?) => {
        impl Accent {
            /// Normalize an accent to a combining one.
            pub fn combine(c: char) -> Option<char> {
                Self::combining(c.encode_utf8(&mut [0; 4])).map(|v| v.0)
            }

            /// Tries to select a well-known combining accent that matches for the
            /// value.
            pub fn combining(value: &str) -> Option<Self> {
                Some(match value {
                    $($($option)|* => Accent($primary),)*
                    _ => return None,
                })
            }
        }

        $(
            /// The accent function for callable symbol definitions.
            #[func]
            pub fn $name(
                /// The base to which the accent is applied.
                base: Content,
                /// The size of the accent, relative to the width of the base.
                #[named]
                size: Option<Rel<Length>>,
                /// Whether to remove the dot on top of lowercase i and j when
                /// adding a top accent.
                #[named]
                dotless: Option<bool>,
            ) -> Content {
                let mut accent = AccentElem::new(base, Accent::new($primary));
                if let Some(size) = size {
                    accent = accent.with_size(size);
                }
                if let Some(dotless) = dotless {
                    accent = accent.with_dotless(dotless);
                }
                accent.pack()
            }
        )+
    };
}

// Keep it synced with the documenting table above.
accents! {
    // Note: Symbols that can have a text presentation must explicitly have that
    // alternative listed here.
    '\u{0300}', "\u{0300}" | "`" => grave,
    '\u{0301}', "\u{0301}" | "´" => acute,
    '\u{0302}', "\u{0302}" | "^" | "ˆ" => hat,
    '\u{0303}', "\u{0303}" | "~" | "∼" | "˜" => tilde,
    '\u{0304}', "\u{0304}" | "¯" => macron,
    '\u{0305}', "\u{0305}" | "-" | "‾" | "−" => dash,
    '\u{0306}', "\u{0306}" | "˘" => breve,
    '\u{0307}', "\u{0307}" | "." | "˙" | "⋅" => dot,
    '\u{0308}', "\u{0308}" | "¨" => dot_double,
    '\u{20db}', "\u{20db}" => dot_triple,
    '\u{20dc}', "\u{20dc}" => dot_quad,
    '\u{030a}', "\u{030a}" | "∘" | "○" => circle,
    '\u{030b}', "\u{030b}" | "˝" => acute_double,
    '\u{030c}', "\u{030c}" | "ˇ" => caron,
    '\u{20d6}', "\u{20d6}" | "←" => arrow_l,
    '\u{20d7}', "\u{20d7}" | "→" | "⟶" => arrow,
    '\u{20e1}', "\u{20e1}" | "↔" | "↔\u{fe0e}" | "⟷" => arrow_l_r,
    '\u{20d0}', "\u{20d0}" | "↼" => harpoon_lt,
    '\u{20d1}', "\u{20d1}" | "⇀" => harpoon,
}

cast! {
    Accent,
    self => self.0.into_value(),
    // The string cast handles
    // - strings: `accent(a, "↔")`
    // - symbol values: `accent(a, <->)`
    // - shorthands: `accent(a, arrow.l.r)`
    v: Str => Self::normalize(&v).ok_or("expected exactly one character")?,
    // The content cast is for accent uses like `accent(a, ↔)`
    v: Content => v.to_packed::<SymbolElem>()
        .and_then(|elem| Accent::normalize(&elem.text))
        .ok_or("expected a single-codepoint symbol")?,
}
