use std::{collections::BTreeMap, sync::Arc};

use bitflags::bitflags;
use comemo::Tracked;
use ecow::{EcoString, EcoVec, eco_vec};
use std::hash::Hash;
use syntect::highlighting::FontStyle;
use typst_syntax::{Span, Spanned};
use typst_utils::ManuallyHash;

use crate::{
    World,
    diag::{LoadedWithin, SourceDiagnostic, SourceResult},
    foundations::{Derived, Dict, FromValue, IntoValue, OneOrMultiple, Packed, Reflect},
    loading::{DataSource, Load as _},
    visualize::Color,
};

#[derive(Clone)]
pub struct TreeSitterHighlightConfiguration {
    /// These names can also refer to this highlight configuration
    aliases: Box<[EcoString]>,
    /// Contains the actual highlight configuration, which tells us how to
    /// do syntax highlighting for the particular language
    config: Arc<ManuallyHash<tree_sitter_highlight::HighlightConfiguration>>,
}

impl TreeSitterHighlightConfiguration {
    /// Load syntaxes from sources.
    pub(crate) fn load(
        world: Tracked<dyn World + '_>,
        syntaxes: Spanned<OneOrMultiple<TreeSitterSyntax>>,
    ) -> SourceResult<
        Derived<OneOrMultiple<TreeSitterSyntax>, Vec<TreeSitterHighlightConfiguration>>,
    > {
        let configurations = syntaxes
            .v
            .0
            .iter()
            .map(|syntax| syntax.load(world, syntaxes.span))
            .collect::<SourceResult<_>>()?;

        Ok(Derived::new(syntaxes.v, configurations))
    }

    /// If the language string matches this language configuration
    pub(crate) fn matches(&self, name: &str) -> bool {
        self.config.language_name == name
            || self.aliases.iter().any(|alias| alias == name)
    }
}

impl PartialEq for TreeSitterHighlightConfiguration {
    fn eq(&self, other: &Self) -> bool {
        self.config.language == other.config.language
    }
}

impl std::hash::Hash for TreeSitterHighlightConfiguration {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.config.language.hash(state);
    }
}

impl std::fmt::Debug for TreeSitterHighlightConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HighlightConfiguration")
            .field(&self.config.language_name)
            .finish()
    }
}

/// WASM Engine is shared across all threads
static ENGINE: std::sync::OnceLock<wasmtime::Engine> = std::sync::OnceLock::new();

thread_local! {
    /// Each thread has its own tree-sitter parser with its own WASM store
    /// That is necessary since when we add new languages to the tree-sitter parser,
    /// we first have to take the WasmStore out, add the language, then add it back in.
    ///
    /// If multiple threads are using the same WasmStore, that approach would cause problems -
    /// as the WasmStore would be missing sometimes, so syntax highlighting would randomly fail
    pub(crate) static PARSER: std::cell::RefCell<tree_sitter::Parser>  = {
        let mut parser = tree_sitter::Parser::new();
        parser.set_wasm_store(tree_sitter::WasmStore::new(ENGINE.get_or_init(Default::default)).unwrap()).unwrap();
        std::cell::RefCell::new(parser)
    };
}

/// Map from names of languages, to parsers of those languages.
/// Multiple names can point to the same language.
#[derive(Default)]
pub struct Languages(
    std::collections::HashMap<String, usize>,
    Vec<tree_sitter::Language>,
);

impl Languages {
    pub fn get(&self, language: &str) -> Option<tree_sitter::Language> {
        self.0.get(language).map(|lang| &self.1[*lang]).cloned()
    }

    pub fn insert(
        &mut self,
        name: String,
        aliases: Box<[String]>,
        language_wasm: &[u8],
    ) -> tree_sitter::Language {
        let language = PARSER.with(|parser| {
            let mut parser = parser.borrow_mut();
            let mut store = parser
                .take_wasm_store()
                .expect("set once during initialization and always re-set after");
            let language = store.load_language(&name, language_wasm).unwrap();
            parser.set_wasm_store(store).expect("succeded during initialization");
            language
        });

        let index = self.1.len();
        self.1.push(language.clone());
        self.0.insert(name, index);
        for alias in aliases {
            self.0.insert(alias, index);
        }
        language
    }
}

bitflags! {
    /// Attributes of syntax-highlighted text
    #[derive(Default, Copy, Clone, Debug, PartialEq, Hash)]
    pub struct FontAttributes: u8 {
        const BOLD   = 0b00000001;
        const ITALIC = 0b00000010;
    }
}

impl From<FontAttributes> for FontStyle {
    fn from(attrs: FontAttributes) -> Self {
        let mut font_style = FontStyle::empty();
        if attrs.contains(FontAttributes::BOLD) {
            font_style.insert(FontStyle::BOLD);
        }
        if attrs.contains(FontAttributes::ITALIC) {
            font_style.insert(FontStyle::ITALIC);
        }
        font_style
    }
}

/// Style of text highlighted by tree-sitter
#[derive(std::hash::Hash, Clone, Debug, PartialEq, Copy, Default)]
pub struct TreeSitterStyle {
    foreground: Option<Color>,
    background: Option<Color>,
    underline: Option<Color>,
    attributes: FontAttributes,
}

impl Reflect for TreeSitterStyle {
    fn input() -> crate::foundations::CastInfo {
        Dict::input() + Color::input()
    }

    fn output() -> crate::foundations::CastInfo {
        Dict::output() + Color::output()
    }

    fn castable(value: &crate::foundations::Value) -> bool {
        Dict::castable(value) || Color::castable(value)
    }
}

impl IntoValue for TreeSitterStyle {
    fn into_value(self) -> crate::foundations::Value {
        crate::foundations::dict! {
            "background" => self.background,
            "foreground" => self.foreground,
            "bold" => self.attributes.contains(FontAttributes::BOLD),
            "underline" => self.underline,
            "italic" => self.attributes.contains(FontAttributes::ITALIC),
        }
        .into_value()
    }
}

impl FromValue for TreeSitterStyle {
    fn from_value(
        value: crate::foundations::Value,
    ) -> crate::diag::HintedStrResult<Self> {
        if Color::castable(&value) {
            let foreground = value.cast().expect("just checked that it's castable");
            return Ok(Self {
                foreground,
                background: None,
                underline: None,
                attributes: FontAttributes::default(),
            });
        }

        let mut dict = value.cast::<Dict>()?;
        let mut attributes = FontAttributes::default();

        if let Ok(bold) = dict.take("bold")
            && bold.cast()?
        {
            attributes.insert(FontAttributes::BOLD);
        }
        if let Ok(italic) = dict.take("italic")
            && italic.cast()?
        {
            attributes.insert(FontAttributes::ITALIC);
        }

        Ok(Self {
            foreground: dict.take("foreground").unwrap_or_default().cast()?,
            background: dict.take("background").unwrap_or_default().cast()?,
            underline: dict.take("underline").unwrap_or_default().cast()?,
            attributes,
        })
    }
}

impl TreeSitterStyle {
    fn into_syntect_style(
        self,
        default_foreground: syntect::highlighting::Color,
    ) -> syntect::highlighting::Style {
        let foreground = self.foreground.map_or(default_foreground, |bg| {
            let fg = bg.to_rgb();
            syntect::highlighting::Color {
                r: (fg.red * 255.0).round() as u8,
                g: (fg.green * 255.0).round() as u8,
                b: (fg.blue * 255.0).round() as u8,
                a: (fg.alpha * 255.0).round() as u8,
            }
        });
        let background = self.background.map_or(
            syntect::highlighting::Color { r: 0, g: 0, b: 0, a: 0 },
            |bg| {
                let bg = bg.to_rgb();
                syntect::highlighting::Color {
                    r: (bg.red * 255.0).round() as u8,
                    g: (bg.green * 255.0).round() as u8,
                    b: (bg.blue * 255.0).round() as u8,
                    a: (bg.alpha * 255.0).round() as u8,
                }
            },
        );
        syntect::highlighting::Style {
            foreground,
            background,
            font_style: self.attributes.into(),
        }
    }
}

/// The syntax highlighting theme to use
#[derive(std::hash::Hash, Clone, Debug, PartialEq)]
pub struct TreeSitterTheme(BTreeMap<EcoString, TreeSitterStyle>);

impl Reflect for TreeSitterTheme {
    fn input() -> crate::foundations::CastInfo {
        Dict::input()
    }

    fn output() -> crate::foundations::CastInfo {
        Dict::output()
    }

    fn castable(value: &crate::foundations::Value) -> bool {
        Dict::castable(value)
    }
}

impl IntoValue for TreeSitterTheme {
    fn into_value(self) -> crate::foundations::Value {
        let mut dict = Dict::new();

        for (key, value) in self.0 {
            dict.insert(key.into(), value.into_value());
        }

        dict.into_value()
    }
}

impl FromValue for TreeSitterTheme {
    fn from_value(
        value: crate::foundations::Value,
    ) -> crate::diag::HintedStrResult<Self> {
        let mut map = BTreeMap::new();

        for (scope, highlight) in value.cast::<Dict>()? {
            map.insert(scope.into(), highlight.cast()?);
        }

        Ok(Self(map))
    }
}

#[derive(Clone, PartialEq, std::hash::Hash, Debug)]
pub struct TreeSitterSyntax {
    /// The tree-sitter grammar in `.wasm` format
    grammar: DataSource,
    /// Name of the tree-sitter grammar
    name: EcoString,
    /// Any aliases of the language
    aliases: Vec<EcoString>,
    /// `highlights.scm` query for syntax highlighting
    highlights_query: Option<DataSource>,
    /// `injections.scm` query for injecting nested grammars
    injections_query: Option<DataSource>,
    /// `locals.scm` query
    locals_query: Option<DataSource>,
}

impl TreeSitterSyntax {
    /// Load syntax from source
    fn load(
        &self,
        world: Tracked<dyn World + '_>,
        span: Span,
    ) -> SourceResult<TreeSitterHighlightConfiguration> {
        let mut errors = EcoVec::new();
        let grammar = Spanned::new(&self.grammar, span).load(world)?;

        let name = self.name.to_string();

        let highlights_query = match self
            .highlights_query
            .as_ref()
            .map(|query| Spanned::new(query, span).load(world))
        {
            Some(Ok(query)) => Some(query),
            Some(Err(errs)) => {
                errors.extend(errs);
                None
            }
            None => None,
        };
        let injections_query = match self
            .injections_query
            .as_ref()
            .map(|query| Spanned::new(query, span).load(world))
        {
            Some(Ok(query)) => Some(query),
            Some(Err(errs)) => {
                errors.extend(errs);
                None
            }
            None => None,
        };
        let locals_query = match self
            .locals_query
            .as_ref()
            .map(|query| Spanned::new(query, span).load(world))
        {
            Some(Ok(query)) => Some(query),
            Some(Err(errs)) => {
                errors.extend(errs);
                None
            }
            None => None,
        };

        let Some(language) = world.tree_sitter_language(
            name.clone(),
            self.aliases.iter().map(Into::into).collect(),
            &grammar.data,
        ) else {
            return Err(eco_vec!(SourceDiagnostic::warning(
                span,
                format!("failed to load tree-sitter grammar for `{name}`")
            )));
        };

        let highlights_query =
            match highlights_query.as_ref().map(|q| q.data.as_str().within(q)) {
                Some(Ok(query)) => query,
                Some(Err(errs)) => {
                    errors.extend(errs);
                    ""
                }
                None => "",
            };
        let injections_query =
            match injections_query.as_ref().map(|q| q.data.as_str().within(q)) {
                Some(Ok(query)) => query,
                Some(Err(errs)) => {
                    errors.extend(errs);
                    ""
                }
                None => "",
            };
        let locals_query = match locals_query.as_ref().map(|q| q.data.as_str().within(q))
        {
            Some(Ok(query)) => query,
            Some(Err(errs)) => {
                errors.extend(errs);
                ""
            }
            None => "",
        };

        let lang_hash = typst_utils::hash128(&language);
        let mut highlight_configuration =
            match tree_sitter_highlight::HighlightConfiguration::new(
                language.clone(),
                &name,
                highlights_query,
                injections_query,
                locals_query,
            ) {
                Ok(conf) => conf,
                Err(error) => {
                    errors.push(crate::diag::SourceDiagnostic::warning(
                        span,
                        format!(
                            "failed to parse tree-sitter {} `{name}`: {error}",
                            "query for language",
                        ),
                    ));
                    return Err(errors);
                }
            };

        if !errors.is_empty() {
            return Err(errors);
        }

        highlight_configuration.configure(SCOPES);

        let highlight_configuration = TreeSitterHighlightConfiguration {
            aliases: self.aliases.iter().map(Into::into).collect(),
            config: std::sync::Arc::new(typst_utils::ManuallyHash::new(
                highlight_configuration,
                lang_hash,
            )),
        };

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(highlight_configuration)
    }
}

impl Reflect for TreeSitterSyntax {
    fn input() -> crate::foundations::CastInfo {
        Dict::input()
    }

    fn output() -> crate::foundations::CastInfo {
        Dict::output()
    }

    fn castable(value: &crate::foundations::Value) -> bool {
        Dict::castable(value)
    }
}

impl IntoValue for TreeSitterSyntax {
    fn into_value(self) -> crate::foundations::Value {
        crate::foundations::dict! {
            "grammar" => self.grammar,
            "name" => self.name,
            "aliases" => self.aliases,
            "highlights-query" => self.highlights_query,
            "injections-query" => self.injections_query,
            "locals-query" => self.locals_query
        }
        .into_value()
    }
}

impl FromValue for TreeSitterSyntax {
    fn from_value(
        value: crate::foundations::Value,
    ) -> crate::diag::HintedStrResult<Self> {
        let mut dict = value.cast::<Dict>()?;

        Ok(Self {
            name: dict.take("name")?.cast()?,
            aliases: dict
                .take("aliases")
                .unwrap_or_else(|_| {
                    crate::foundations::Value::Array(crate::foundations::Array::default())
                })
                .cast()?,
            grammar: dict.take("grammar")?.cast()?,
            highlights_query: dict.take("highlights-query").unwrap_or_default().cast()?,
            injections_query: dict.take("injections-query").unwrap_or_default().cast()?,
            locals_query: dict.take("locals-query").unwrap_or_default().cast()?,
        })
    }
}

/// Highlight `lines` of text using the tree-sitter `syntax` with the
/// color `theme`
#[allow(clippy::too_many_arguments)]
pub(crate) fn highlight(
    all_configs: &[TreeSitterHighlightConfiguration],
    routines: &crate::routines::Routines,
    target: crate::foundations::Target,
    lines: EcoVec<(EcoString, Span)>,
    seq: &mut Vec<Packed<super::RawLine>>,
    foreground: syntect::highlighting::Color,
    count: i64,
    syntax: &TreeSitterHighlightConfiguration,
    theme: &TreeSitterTheme,
) {
    let parser = PARSER.take();
    let mut highlighter = tree_sitter_highlight::Highlighter::new();
    highlighter.parser = parser;

    // Whole text of the code block
    let text = lines.iter().map(|(s, _)| s.clone()).collect::<Vec<_>>().join("\n");

    let mut current_highlight = None;

    // Text of the code block, broken up into individual
    // string slices associated with their style
    let mut pieces = Vec::new();

    for event in highlighter
        .highlight(&syntax.config, text.as_bytes(), None, |lang| {
            all_configs
                .iter()
                .find(|config| config.matches(lang))
                .map(|it| &**it.config)
        })
        .unwrap()
    {
        match event.unwrap() {
            tree_sitter_highlight::HighlightEvent::Source { start, end } => {
                pieces.push((&text[start..end], current_highlight));
            }
            tree_sitter_highlight::HighlightEvent::HighlightStart(highlight) => {
                let scope = SCOPES[highlight.0];

                // For a string like "foo.bar.baz", we want to check if "foo.bar.baz" is a valid
                // key. If not, check "foo.bar". If not, check "foo"

                let mut current_scope = Vec::new();

                // List of all the scopes we'll check at the end
                let mut all_scopes = Vec::new();

                for part in scope.split(".") {
                    current_scope.push(part);
                    all_scopes.push(current_scope.join("."));
                }

                let color = all_scopes
                    .into_iter()
                    .rev()
                    .find_map(|scope| theme.0.get(&*scope))
                    .copied()
                    .unwrap_or_default();

                current_highlight = Some(color);
            }
            tree_sitter_highlight::HighlightEvent::HighlightEnd => {
                current_highlight = None;
            }
        }
    }

    let mut chars = Vec::new();

    for (piece, style) in pieces {
        for char in piece.chars() {
            chars.push((style, char));
        }
    }

    let mut highlighted_lines = Vec::new();
    let mut current_line = Vec::new();
    for (style, char) in chars {
        if char == '\n' {
            highlighted_lines.push(current_line.clone());
            current_line.clear();
        } else {
            current_line.push((style, char));
        }
    }
    if !current_line.is_empty() {
        highlighted_lines.push(current_line);
    }

    for ((i, line), (line_string, line_span)) in
        highlighted_lines.into_iter().enumerate().zip(lines)
    {
        let mut line_content = Vec::new();
        let mut span_offset = 0;
        for (style, piece) in line {
            let piece = piece.to_string();
            line_content.push(crate::text::styled(
                routines,
                target,
                &piece,
                foreground,
                // If its `None`, then it is a whitespace character
                style.unwrap_or_default().into_syntect_style(foreground),
                line_span,
                span_offset,
            ));
            span_offset += piece.len();
        }

        seq.push(
            Packed::new(super::RawLine::new(
                i as i64 + 1,
                count,
                line_string,
                crate::foundations::Content::sequence(line_content),
            ))
            .spanned(line_span),
        );
    }
    PARSER.set(highlighter.parser);
}

/// Supports all the same scopes as Helix.
/// Taken from <https://docs.helix-editor.com/themes.html#syntax-highlighting>
///
/// By using Helix's scopes, we are leveraging the hundreds of available
/// language configurations + the hundreds of available themes.
///
/// Neovim nor Zed, nor any other source come close to the amount of
/// languages/themes Helix supports and how easy it is to turn a Helix
/// theme to be supported by Typst - as all Helix themes are just static .toml files
pub const SCOPES: &[&str] = &[
    "attribute",
    "type",
    "type.builtin",
    "type.parameter",
    "type.enum",
    "type.enum.variant",
    "constructor",
    "constant",
    "constant.builtin",
    "constant.builtin.boolean",
    "constant.character",
    "constant.character.escape",
    "constant.numeric",
    "constant.numeric.integer",
    "constant.numeric.float",
    "string",
    "string.regexp",
    "string.special",
    "string.special.path",
    "string.special.url",
    "string.special.symbol",
    "comment",
    "comment.line",
    "comment.line.documentation",
    "comment.block",
    "comment.block.documentation",
    "comment.unused",
    "variable",
    "variable.builtin",
    "variable.parameter",
    "variable.other",
    "variable.other.member",
    "variable.other.member.private",
    "label",
    "punctuation",
    "punctuation.delimiter",
    "punctuation.bracket",
    "punctuation.special",
    "keyword",
    "keyword.control",
    "keyword.control.conditional",
    "keyword.control.repeat",
    "keyword.control.import",
    "keyword.control.return",
    "keyword.control.exception",
    "keyword.operator",
    "keyword.directive",
    "keyword.function",
    "keyword.storage",
    "keyword.storage.type",
    "keyword.storage.modifier",
    "operator",
    "function",
    "function.builtin",
    "function.method",
    "function.method.private",
    "function.macro",
    "function.special",
    "tag",
    "tag.builtin",
    "namespace",
    "special",
    "markup",
    "markup.heading",
    "markup.heading.marker",
    "markup.heading.h1",
    "markup.heading.h2",
    "markup.heading.h3",
    "markup.heading.h4",
    "markup.heading.h5",
    "markup.heading.h6",
    "markup.list",
    "markup.list.unnumbered",
    "markup.list.numbered",
    "markup.list.checked",
    "markup.list.unchecked",
    "markup.bold",
    "markup.italic",
    "markup.strikethrough",
    "markup.link",
    "markup.link.url",
    "markup.link.label",
    "markup.link.text",
    "markup.quote",
    "markup.raw",
    "markup.raw.inline",
    "markup.raw.block",
    "diff",
    "diff.plus",
    "diff.plus.gutter",
    "diff.minus",
    "diff.minus.gutter",
    "diff.delta",
    "diff.delta.moved",
    "diff.delta.conflict",
    "diff.delta.gutter",
];
