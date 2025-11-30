use std::collections::HashMap;
use std::sync::LazyLock;

use bumpalo::Bump;
use comemo::Tracked;
use icu_properties::CanonicalCombiningClass;
use icu_properties::maps::CodePointMapData;
use icu_provider::AsDeserializingBufferProvider;
use icu_provider_blob::BlobDataProvider;

use crate::diag::bail;
use crate::engine::Engine;
use crate::foundations::{
    Args, CastInfo, Content, Context, Func, IntoValue, NativeElement, NativeFuncData,
    NativeFuncPtr, ParamInfo, Reflect, Scope, SymbolElem, Type, cast, elem,
};
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

    /// Normalize an accent to a combining one.
    pub fn combine(c: char) -> Option<char> {
        ACCENTS
            .iter()
            .copied()
            .find(|(accent, names)| *accent == c || names.contains(&c))
            .map(|(accent, _)| accent)
    }

    /// Whether this accent is a bottom accent or not.
    pub fn is_bottom(&self) -> bool {
        if matches!(self.0, '⏟' | '⎵' | '⏝' | '⏡') {
            return true;
        }

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

/// Gets the accent function corresponding to a name, if any.
pub fn get_accent_func(c: char) -> Option<Func> {
    Some(
        FUNCS
            .get(&Accent::combine(c)?)
            .expect("this accent should have a function")
            .into(),
    )
}

// Keep it synced with the documenting table above.
/// A list of accents, each with a list of alternative names.
const ACCENTS: &[(char, &[char])] = &[
    ('\u{0300}', &['`']),
    ('\u{0301}', &['´']),
    ('\u{0302}', &['^', 'ˆ']),
    ('\u{0303}', &['~', '∼', '˜']),
    ('\u{0304}', &['¯']),
    ('\u{0305}', &['-', '–', '‾', '−']),
    ('\u{0306}', &['˘']),
    ('\u{0307}', &['.', '˙', '⋅']),
    ('\u{0308}', &['¨']),
    ('\u{20db}', &[]),
    ('\u{20dc}', &[]),
    ('\u{030a}', &['∘', '○']),
    ('\u{030b}', &['˝']),
    ('\u{030c}', &['ˇ']),
    ('\u{20d6}', &['←']),
    ('\u{20d7}', &['→', '⟶']),
    ('\u{20e1}', &['↔', '⟷']),
    ('\u{20d0}', &['↼']),
    ('\u{20d1}', &['⇀']),
];

/// Lazily created accent functions.
static FUNCS: LazyLock<HashMap<char, NativeFuncData>> = LazyLock::new(|| {
    let bump = Box::leak(Box::new(Bump::new()));
    ACCENTS
        .iter()
        .copied()
        .map(|(accent, _)| (accent, create_accent_func_data(accent, bump)))
        .collect()
});

/// Creates metadata for an accent wrapper function.
fn create_accent_func_data(accent: char, bump: &'static Bump) -> NativeFuncData {
    let title = bumpalo::format!(in bump, "Accent ({})", accent).into_bump_str();
    let docs = bumpalo::format!(in bump, "Adds the accent {} on an expression.", accent)
        .into_bump_str();
    NativeFuncData {
        function: NativeFuncPtr(bump.alloc(
            move |_: &mut Engine, _: Tracked<Context>, args: &mut Args| {
                let base = args.expect("base")?;
                let size = args.named("size")?;
                let dotless = args.named("dotless")?;
                let mut elem = AccentElem::new(base, Accent(accent));
                if let Some(size) = size {
                    elem = elem.with_size(size);
                }
                if let Some(dotless) = dotless {
                    elem = elem.with_dotless(dotless);
                }
                Ok(elem.pack().into_value())
            },
        )),
        name: "(..) => ..",
        title,
        docs,
        keywords: &[],
        contextual: false,
        scope: LazyLock::new(&|| Scope::new()),
        params: LazyLock::new(&|| create_accent_param_info()),
        returns: LazyLock::new(&|| CastInfo::Type(Type::of::<Content>())),
    }
}

/// Creates parameter signature metadata for an accent function.
fn create_accent_param_info() -> Vec<ParamInfo> {
    vec![
        ParamInfo {
            name: "base",
            docs: "The base to which the accent is applied.",
            input: Content::input(),
            default: None,
            positional: true,
            named: false,
            variadic: false,
            required: true,
            settable: false,
        },
        ParamInfo {
            name: "size",
            docs: "The size of the accent, relative to the width of the base.",
            input: Rel::<Length>::input(),
            default: None,
            positional: false,
            named: true,
            variadic: false,
            required: false,
            settable: false,
        },
        ParamInfo {
            name: "dotless",
            docs: "Whether to remove the dot on top of lowercase i and j when adding a top accent.",
            input: bool::input(),
            default: None,
            positional: false,
            named: true,
            variadic: false,
            required: false,
            settable: false,
        },
    ]
}

cast! {
    Accent,
    self => self.0.into_value(),
    v: char => Self::new(v),
    v: Content => match v.to_packed::<SymbolElem>().and_then(|elem| elem.text.parse::<char>().ok()) {
        Some(c) => Self::new(c),
        _ => bail!("expected a single-codepoint symbol"),
    },
}
