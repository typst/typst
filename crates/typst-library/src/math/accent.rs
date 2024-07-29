use crate::diag::bail;
use crate::foundations::{cast, elem, func, Content, NativeElement, Value};
use crate::layout::{Length, Rel};
use crate::math::{Mathy, VarElem};

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
    /// The base to which the accent is applied.
    /// May consist of multiple letters.
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
    #[resolve]
    #[default(Rel::one())]
    pub size: Rel<Length>,
}

/// An accent character.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Accent(pub char);

impl Accent {
    /// Normalize a character into an accent.
    pub fn new(c: char) -> Self {
        Self(Self::combine(c).unwrap_or(c))
    }
}

/// This macro generates accent-related functions.
///
/// ```ignore
/// accents! {
///     '\u{0300}' | '`' => grave,
/// //  ^^^^^^^^^    ^^^    ^^^^^
/// //  |            |      |
/// //  |            |      +-- The name of the function.
/// //  |            +--------- The alternative characters that represent the accent.
/// //  +---------------------- The primary character that represents the accent.
/// }
/// ```
///
/// When combined with the `Accent::combine` function, accent characters can be normalized
/// to the primary character.
macro_rules! accents {
    ($($primary:literal $(| $alt:literal)* => $name:ident),* $(,)?) => {
        impl Accent {
            /// Normalize an accent to a combining one.
            pub fn combine(c: char) -> Option<char> {
                Some(match c {
                    $($primary $(| $alt)* => $primary,)*
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
            ) -> Content {
                let mut accent = AccentElem::new(base, Accent::new($primary));
                if let Some(size) = size {
                    accent = accent.with_size(size);
                }
                accent.pack()
            }
        )+
    };
}

// Keep it synced with the documenting table above.
accents! {
    '\u{0300}' | '`' => grave,
    '\u{0301}' | '´' => acute,
    '\u{0302}' | '^' | 'ˆ' => hat,
    '\u{0303}' | '~' | '∼' | '˜' => tilde,
    '\u{0304}' | '¯' => macron,
    '\u{0305}' | '-' | '‾' | '−' => dash,
    '\u{0306}' | '˘' => breve,
    '\u{0307}' | '.' | '˙' | '⋅' => dot,
    '\u{0308}' | '¨' => dot_double,
    '\u{20db}' => dot_triple,
    '\u{20dc}' => dot_quad,
    '\u{030a}' | '∘' | '○' => circle,
    '\u{030b}' | '˝' => acute_double,
    '\u{030c}' | 'ˇ' => caron,
    '\u{20d6}' | '←' => arrow_l,
    '\u{20d7}' | '→' | '⟶' => arrow,
    '\u{20e1}' | '↔' | '⟷' => arrow_l_r,
    '\u{20d0}' | '↼' => harpoon_lt,
    '\u{20d1}' | '⇀' => harpoon,
}

cast! {
    Accent,
    self => self.0.into_value(),
    v: char => Self::new(v),
    v: Content => match v.to_packed::<VarElem>() {
        Some(elem) => Value::Str(elem.text.clone().into()).cast()?,
        None => bail!("expected text"),
    },
}
