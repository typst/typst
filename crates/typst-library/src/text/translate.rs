use std::collections::HashMap;
use typst::syntax;

use crate::prelude::*;
use crate::text::TextElem;

#[derive(Debug)]
pub enum Translations {
    Dict(HashMap<Lang, Content>),
    Func(Func),
}

impl Translations {
    pub fn value_in(
        &self,
        vt: &mut Vt,
        lang: Lang,
        region: Option<Region>,
    ) -> SourceResult<Option<Content>> {
        match self {
            Self::Dict(languages) => Ok(languages.get(&lang).cloned()),
            Self::Func(func) => Ok(Some(
                func.call_vt(vt, [lang.into_value(), region.into_value()])?.display(),
            )),
        }
    }
}

cast! {
    Translations,

    self => match self {
        Self::Dict(v) => Value::Dict(
            v.into_iter()
            .map(|(lang, value)| (lang.as_str().into(), value.into_value()))
            .collect()
        ),
        Self::Func(v) => Value::Func(v),
    },

    v: Dict => Self::Dict(
        v.into_iter()
        .map(|(lang, value)| Ok((lang.parse()?, Content::from_value(value)?)))
        .collect::<StrResult<_>>()?
    ),
    v: Func => Self::Func(v),
}

/// An element that renders different content depending on the locale.
///
/// Useful for creating macros that generate text.
///
/// ```example
/// #let theorem = translate((
///   ja: "定理",
///   sv: "Teorem",
/// ))
/// #set text(lang: "ja")
/// #theorem
/// #set text(lang: "sv")
/// #theorem
/// ```
#[elem(Show)]
pub struct TranslateElem {
    /// The translations.
    ///
    /// May either be a dictionary whose keys are [language codes]($text.lang)
    /// and values are the corresponding translations, or a function that
    /// accepts a [language code]($text.lang) and a [region code]($text.region)
    /// and returns content.
    ///
    /// Note that language codes are always lowercase, while region codes are
    /// always uppercase, and may be `{none}`.
    ///
    /// ```example
    /// #let locale = translate(
    ///   (lang, region) => {
    ///     if region != none { lang }
    ///     else [#(lang)-#region]
    ///   }
    /// )
    /// #set text(lang: "fr", region: "CA")
    /// #locale
    /// ```
    #[required]
    pub translations: Translations,
}

impl Show for TranslateElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let lang = TextElem::lang_in(styles);
        let region = TextElem::region_in(styles);
        if let Some(value) = self.translations().value_in(vt, lang, region)? {
            Ok(value)
        } else {
            let locale = if let Some(region) = region {
                eco_format!("{}-{}", lang.as_str(), region.as_str())
            } else {
                lang.as_str().into()
            };
            bail!(syntax::Span::detached(), "No translation available for {}", locale)
        }
    }
}
