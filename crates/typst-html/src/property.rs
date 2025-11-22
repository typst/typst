//! Typed CSS properties.
//!
//! Sourced from the HTML and CSS specifications, but also sometimes informed
//! by real-world user agent style sheets:
//! - <https://searchfox.org/firefox-main/rev/33682acc1fa0db34ac826b143048db28ee9f16a6/layout/style/res/>
//! - <https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/html/resources/html.css;drc=25f499fd1faae687309ba0a07b23798efcc099a8>

use crate::{HtmlTag, tag};

/// A value for the CSS `display` property.
///
/// <https://www.w3.org/TR/css-display-3/#propdef-display>
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Display {
    // <display-outside>
    Block,
    Inline,
    RunIn,

    // <display-inside>
    Flow,
    FlowRoot,
    Table,
    Flex,
    Grid,
    Ruby,

    // <display-listitem>
    ListItem,

    // <display-internal>
    TableRowGroup,
    TableHeaderGroup,
    TableFooterGroup,
    TableRow,
    TableCell,
    TableColumnGroup,
    TableColumn,
    TableCaption,
    RubyBase,
    RubyText,

    // <display-box>
    Contents,
    None,

    // <display-legacy>
    InlineBlock,
    InlineTable,
    InlineFlex,
    InlineGrid,
}

impl Display {
    /// Returns the default value for the given tag as defined by the user agent
    /// styles in § 15 of the HTML spec.
    pub fn default_for(tag: HtmlTag) -> Option<Self> {
        Some(match tag {
            // § 15.3.1 Hidden elements.
            tag::area => Self::None,
            tag::base => Self::None,
            tag::datalist => Self::None,
            tag::head => Self::None,
            tag::link => Self::None,
            tag::meta => Self::None,
            tag::rp => Self::None,
            tag::script => Self::None,
            tag::style => Self::None,
            tag::template => Self::None,
            tag::title => Self::None,

            // § 15.3.2 The page.
            tag::html => Self::Block,
            tag::body => Self::Block,

            // § 15.3.3 Flow content.
            tag::address => Self::Block,
            tag::blockquote => Self::Block,
            tag::dialog => Self::Block,
            tag::div => Self::Block,
            tag::figure => Self::Block,
            tag::figcaption => Self::Block,
            tag::footer => Self::Block,
            tag::form => Self::Block,
            tag::header => Self::Block,
            tag::hr => Self::Block,
            tag::legend => Self::Block,
            tag::main => Self::Block,
            tag::p => Self::Block,
            tag::pre => Self::Block,
            tag::search => Self::Block,
            tag::slot => Self::Contents,

            // § 15.3.4 Phrasing content.
            tag::ruby => Self::Ruby,
            tag::rt => Self::RubyText,

            // § 15.3.6 Sections and headings.
            tag::article => Self::Block,
            tag::aside => Self::Block,
            tag::h1 => Self::Block,
            tag::h2 => Self::Block,
            tag::h3 => Self::Block,
            tag::h4 => Self::Block,
            tag::h5 => Self::Block,
            tag::h6 => Self::Block,
            tag::hgroup => Self::Block,
            tag::nav => Self::Block,
            tag::section => Self::Block,

            // § 15.3.7 Lists.
            tag::dd => Self::Block,
            tag::dl => Self::Block,
            tag::dt => Self::Block,
            tag::menu => Self::Block,
            tag::ol => Self::Block,
            tag::ul => Self::Block,
            tag::li => Self::ListItem,

            // § 15.3.8 Tables.
            tag::table => Self::Table,
            tag::thead => Self::TableHeaderGroup,
            tag::tbody => Self::TableRowGroup,
            tag::tfoot => Self::TableFooterGroup,
            tag::tr => Self::TableRow,
            tag::th => Self::TableCell,
            tag::td => Self::TableCell,
            tag::caption => Self::TableCaption,
            tag::col => Self::TableColumn,
            tag::colgroup => Self::TableColumnGroup,

            // § 15.3.10 Form controls.
            tag::input => Self::InlineBlock,
            tag::button => Self::InlineBlock,

            // § 15.3.12 The fieldset and legend elements.
            tag::fieldset => Self::Block,

            // § 15.5.5 The details and summary elements.
            tag::details => Self::Block,
            tag::summary => Self::Block,

            // § 15.5.14 The meter element.
            //
            // Defined in free text rather than in a CSS snippet.
            tag::meter => Self::InlineBlock,

            // § 15.5.15 The progress element.
            //
            // Defined in free text rather than in a CSS snippet.
            tag::progress => Self::InlineBlock,

            // § 15.5.16 The select element.
            //
            // The spec is silent on `option` and only specifies a value for
            // `select optgroup`, but UA style sheets specify `display: block`.
            tag::select => Self::InlineBlock,
            tag::option => Self::Block,
            tag::optgroup => Self::Block,

            // § 15.5.17 The textarea element.
            //
            // Defined in free text rather than in a CSS snippet.
            tag::textarea => Self::InlineBlock,

            // `display: inline` is the default of the CSS property.
            tag::a => Self::Inline,
            tag::abbr => Self::Inline,
            tag::audio => Self::Inline,
            tag::b => Self::Inline,
            tag::bdi => Self::Inline,
            tag::bdo => Self::Inline,
            tag::br => Self::Inline,
            tag::canvas => Self::Inline,
            tag::cite => Self::Inline,
            tag::code => Self::Inline,
            tag::data => Self::Inline,
            tag::del => Self::Inline,
            tag::dfn => Self::Inline,
            tag::em => Self::Inline,
            tag::embed => Self::Inline,
            tag::i => Self::Inline,
            tag::iframe => Self::Inline,
            tag::img => Self::Inline,
            tag::ins => Self::Inline,
            tag::kbd => Self::Inline,
            tag::label => Self::Inline,
            tag::map => Self::Inline,
            tag::mark => Self::Inline,
            tag::noscript => Self::Inline,
            tag::object => Self::Inline,
            tag::output => Self::Inline,
            tag::picture => Self::Inline,
            tag::q => Self::Inline,
            tag::s => Self::Inline,
            tag::samp => Self::Inline,
            tag::small => Self::Inline,
            tag::source => Self::Inline,
            tag::span => Self::Inline,
            tag::strong => Self::Inline,
            tag::sub => Self::Inline,
            tag::sup => Self::Inline,
            tag::time => Self::Inline,
            tag::track => Self::Inline,
            tag::u => Self::Inline,
            tag::var => Self::Inline,
            tag::video => Self::Inline,
            tag::wbr => Self::Inline,

            // MathML Core elements.
            tag::mathml::mtable => Self::InlineTable,
            tag::mathml::mtr => Self::TableRow,
            tag::mathml::mtd => Self::TableCell,

            // We don't make any assumptions about unknown elements.
            _ => return None,
        })
    }

    /// Whether this is any of the `table(-.*)?` display modes.
    pub fn is_tabular(self) -> bool {
        matches!(
            self,
            Self::Table
                | Self::TableCaption
                | Self::TableColumn
                | Self::TableColumnGroup
                | Self::TableRow
                | Self::TableRowGroup
                | Self::TableHeaderGroup
                | Self::TableFooterGroup
                | Self::TableCell
        )
    }

    /// The CSS identifier of the value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Inline => "inline",
            Self::RunIn => "run-in",
            Self::Flow => "flow",
            Self::FlowRoot => "flow-root",
            Self::Table => "table",
            Self::Flex => "flex",
            Self::Grid => "grid",
            Self::Ruby => "ruby",
            Self::ListItem => "list-item",
            Self::TableRowGroup => "table-row-group",
            Self::TableHeaderGroup => "table-header-group",
            Self::TableFooterGroup => "table-footer-group",
            Self::TableRow => "table-row",
            Self::TableCell => "table-cell",
            Self::TableColumnGroup => "table-column-group",
            Self::TableColumn => "table-column",
            Self::TableCaption => "table-caption",
            Self::RubyBase => "ruby-base",
            Self::RubyText => "ruby-text",
            Self::Contents => "contents",
            Self::None => "none",
            Self::InlineBlock => "inline-block",
            Self::InlineTable => "inline-table",
            Self::InlineFlex => "inline-flex",
            Self::InlineGrid => "inline-grid",
        }
    }
}
