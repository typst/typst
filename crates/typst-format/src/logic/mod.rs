mod code;
mod markup;
mod math;

use code::*;
use markup::*;
use math::*;

use typst_syntax::{SyntaxKind, SyntaxNode};

use crate::{
    output::{Output, OutputTarget, PositionCalculator, Priority, Whitespace},
    settings::*,
    state::{Mode, State},
};

pub fn format(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    if node.erroneous()
        && node
            .children()
            .flat_map(|child| child.children())
            .any(|child| child.kind() == SyntaxKind::Error)
    {
        return skip_formatting(node, state, settings, output);
    }

    match node.kind() {
        SyntaxKind::Markup => format_markup(node, state, settings, output),
        SyntaxKind::Text => format_default(node, state, settings, output),
        SyntaxKind::Space => format_space(node, state, settings, output),
        SyntaxKind::Linebreak => format_and_new_line(node, state, settings, output),
        SyntaxKind::Parbreak => {
            output.set_whitespace(Whitespace::LineBreaks(2), Priority::High)
        }
        SyntaxKind::Escape => format_default(node, state, settings, output),
        SyntaxKind::Shorthand => format_default(node, state, settings, output),
        SyntaxKind::SmartQuote => format_default(node, state, settings, output),
        SyntaxKind::Strong => format_default(node, state, settings, output),
        SyntaxKind::Emph => format_default(node, state, settings, output),
        SyntaxKind::Raw => format_default(node, state, settings, output),
        SyntaxKind::Link => format_default(node, state, settings, output),
        SyntaxKind::Label => format_label(node, state, settings, output),
        SyntaxKind::Ref => format_default(node, state, settings, output),
        SyntaxKind::RefMarker => format_default(node, state, settings, output),
        SyntaxKind::Heading => format_heading(settings, node, state, output),
        SyntaxKind::HeadingMarker => format_default(node, state, settings, output),
        SyntaxKind::ListItem => format_list(node, state, settings, output),
        SyntaxKind::ListMarker => format_default(node, state, settings, output),
        SyntaxKind::EnumItem => format_list(node, state, settings, output),
        SyntaxKind::EnumMarker => format_default(node, state, settings, output),
        SyntaxKind::TermItem => format_term(node, state, settings, output),
        SyntaxKind::TermMarker => format_default(node, state, settings, output),
        SyntaxKind::Equation => format_equation(node, state, settings, output),

        SyntaxKind::Math => format_default(node, state, settings, output),
        SyntaxKind::MathIdent => format_default(node, state, settings, output),
        SyntaxKind::MathAlignPoint => format_default(node, state, settings, output),
        SyntaxKind::MathDelimited => format_default(node, state, settings, output),
        SyntaxKind::MathAttach => format_math_attach(node, state, settings, output),
        SyntaxKind::MathPrimes => format_default(node, state, settings, output),
        SyntaxKind::MathFrac => format_default(node, state, settings, output),
        SyntaxKind::MathRoot => format_default(node, state, settings, output),

        SyntaxKind::Hashtag => output.raw(node, &state, settings),
        SyntaxKind::LeftBrace => output.raw(node, &state, settings),
        SyntaxKind::RightBrace => output.raw(node, &state, settings),
        SyntaxKind::LeftBracket => output.raw(node, &state, settings),
        SyntaxKind::RightBracket => output.raw(node, &state, settings),
        SyntaxKind::LeftParen => output.raw(node, &state, settings),
        SyntaxKind::RightParen => output.raw(node, &state, settings),
        SyntaxKind::Comma => output.raw(node, &state, settings),
        SyntaxKind::Semicolon => format_semicolon(node, state, settings, output),
        SyntaxKind::Colon => output.raw(node, &state, settings),
        SyntaxKind::Star => format_star(node, state, settings, output),
        SyntaxKind::Underscore => format_underscore(node, state, settings, output),
        SyntaxKind::Dollar => format_default(node, state, settings, output),
        SyntaxKind::Plus => format_padded(node, state, settings, output),
        SyntaxKind::Minus => format_padded(node, state, settings, output),
        SyntaxKind::Slash => format_padded(node, state, settings, output),
        SyntaxKind::Hat => format_default(node, state, settings, output),
        SyntaxKind::Prime => format_default(node, state, settings, output),
        SyntaxKind::Dot => format_no_padding(node, state, settings, output),
        SyntaxKind::Eq => format_padded(node, state, settings, output),
        SyntaxKind::EqEq => format_padded(node, state, settings, output),
        SyntaxKind::ExclEq => format_padded(node, state, settings, output),
        SyntaxKind::Lt => format_padded(node, state, settings, output),
        SyntaxKind::LtEq => format_padded(node, state, settings, output),
        SyntaxKind::Gt => format_padded(node, state, settings, output),
        SyntaxKind::GtEq => format_padded(node, state, settings, output),
        SyntaxKind::PlusEq => format_padded(node, state, settings, output),
        SyntaxKind::HyphEq => format_padded(node, state, settings, output),
        SyntaxKind::StarEq => format_padded(node, state, settings, output),
        SyntaxKind::SlashEq => format_padded(node, state, settings, output),
        SyntaxKind::Dots => format_right_bound(node, state, settings, output),
        SyntaxKind::Arrow => format_padded(node, state, settings, output),
        SyntaxKind::Root => format_right_bound(node, state, settings, output),

        SyntaxKind::Not => output.raw(node, &state, settings),
        SyntaxKind::And => output.raw(node, &state, settings),
        SyntaxKind::Or => output.raw(node, &state, settings),
        SyntaxKind::None => output.raw(node, &state, settings),
        SyntaxKind::Auto => output.raw(node, &state, settings),
        SyntaxKind::Let => output.raw(node, &state, settings),
        SyntaxKind::Set => output.raw(node, &state, settings),
        SyntaxKind::Show => output.raw(node, &state, settings),
        SyntaxKind::If => output.raw(node, &state, settings),
        SyntaxKind::Else => output.raw(node, &state, settings),
        SyntaxKind::For => output.raw(node, &state, settings),
        SyntaxKind::In => output.raw(node, &state, settings),
        SyntaxKind::While => output.raw(node, &state, settings),
        SyntaxKind::Break => output.raw(node, &state, settings),
        SyntaxKind::Continue => output.raw(node, &state, settings),
        SyntaxKind::Return => output.raw(node, &state, settings),
        SyntaxKind::Import => output.raw(node, &state, settings),
        SyntaxKind::Include => output.raw(node, &state, settings),
        SyntaxKind::As => output.raw(node, &state, settings),

        SyntaxKind::Code => format_default(node, state, settings, output),
        SyntaxKind::Ident => format_default(node, state, settings, output),
        SyntaxKind::Bool => format_default(node, state, settings, output),
        SyntaxKind::Int => format_default(node, state, settings, output),
        SyntaxKind::Float => format_default(node, state, settings, output),
        SyntaxKind::Numeric => format_default(node, state, settings, output),
        SyntaxKind::Str => format_default(node, state, settings, output),
        SyntaxKind::CodeBlock => format_code_block(node, state, settings, output),
        SyntaxKind::ContentBlock => format_content_block(node, state, settings, output),
        SyntaxKind::Parenthesized => format_default(node, state, settings, output),
        SyntaxKind::Array => format_items(node, state, settings, output),
        SyntaxKind::Dict => format_items(node, state, settings, output),
        SyntaxKind::Named => format_named_argument(node, state, settings, output),
        SyntaxKind::Keyed => format_keyed(node, state, settings, output),
        SyntaxKind::Unary => format_unary(node, state, settings, output),
        SyntaxKind::Binary => format_default(node, state, settings, output),
        SyntaxKind::FieldAccess => format_default(node, state, settings, output),
        SyntaxKind::FuncCall => format_func_call(node, state, settings, output),
        SyntaxKind::Args => format_items(node, state, settings, output),
        SyntaxKind::Spread => format_default(node, state, settings, output),
        SyntaxKind::Closure => format_default(node, state, settings, output),
        SyntaxKind::Params => format_items(node, state, settings, output),
        SyntaxKind::LetBinding => format_code_statement(node, state, settings, output),
        SyntaxKind::SetRule => format_code_statement(node, state, settings, output),
        SyntaxKind::ShowRule => format_code_statement(node, state, settings, output),
        SyntaxKind::Conditional => format_default(node, state, settings, output),
        SyntaxKind::WhileLoop => format_default(node, state, settings, output),
        SyntaxKind::ForLoop => format_default(node, state, settings, output),
        SyntaxKind::ModuleImport => format_code_statement(node, state, settings, output),
        SyntaxKind::ImportItems => format_default(node, state, settings, output),
        SyntaxKind::ModuleInclude => format_code_statement(node, state, settings, output),
        SyntaxKind::LoopBreak => format_default(node, state, settings, output),
        SyntaxKind::LoopContinue => format_default(node, state, settings, output),
        SyntaxKind::FuncReturn => format_default(node, state, settings, output),
        SyntaxKind::Destructuring => format_items(node, state, settings, output),
        SyntaxKind::DestructAssignment => format_default(node, state, settings, output),
        SyntaxKind::RenamedImportItem => format_default(node, state, settings, output),

        SyntaxKind::LineComment => format_and_new_line(node, state, settings, output),
        SyntaxKind::BlockComment => format_default(node, state, settings, output),
        SyntaxKind::Error => format_default(node, state, settings, output),
        SyntaxKind::Eof => format_end_of_file(node, state, settings, output),
    }
}

fn format_default(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    output.raw(node, &state, settings);
    for child in node.children() {
        format(child, state, settings, output);
    }
}

fn format_no_padding(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    output.set_whitespace(Whitespace::None, Priority::Normal);
    format_default(node, state, settings, output);
    output.set_whitespace(Whitespace::None, Priority::Normal);
}

fn format_padded(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    output.set_whitespace(Whitespace::Space, Priority::Low);
    format_default(node, state, settings, output);
    output.set_whitespace(Whitespace::Space, Priority::Low);
}

fn format_and_new_line(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    format_default(node, state, settings, output);
    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
}

fn format_right_bound(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    format_default(node, state, settings, output);
    output.set_whitespace(Whitespace::None, Priority::Normal);
}

fn skip_formatting(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    output.raw(node, &state, settings);
    for child in node.children() {
        skip_formatting(child, state, settings, output);
    }
}

fn get_length(node: &SyntaxNode, settings: &Settings) -> Option<usize> {
    let mut calculator = PositionCalculator::new();
    let mut output = Output::new(&mut calculator);
    let state = State::new();
    format(node, state, settings, &mut output);
    let (line, column) = output.position();
    if line > 1 {
        None
    } else {
        Some(column)
    }
}

fn format_list(
    node: &SyntaxNode,
    mut state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Markup => {
                state.indent();
                for sub_child in child.children() {
                    match sub_child.kind() {
                        SyntaxKind::ListItem | SyntaxKind::EnumItem => {
                            format(sub_child, state, settings, output);
                        }
                        _ => format(sub_child, state, settings, output),
                    }
                }
                state.dedent();
            }
            _ => format(child, state, settings, output),
        }
    }
    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
}

fn format_space(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let preserve = match state.mode {
        Mode::Code => true,
        Mode::Markdown => settings.preserve_newline.content,
        Mode::Math => settings.preserve_newline.math,
        Mode::Items => false,
    };
    if preserve {
        match node.text().chars().fold(0, |acc, c| acc + (c == '\n') as usize) {
            0 => output.set_whitespace(Whitespace::Space, Priority::Low),
            1 => output.set_whitespace(Whitespace::LineBreak, Priority::Normal),
            _ => output.set_whitespace(Whitespace::LineBreaks(2), Priority::Normal),
        }
    } else {
        output.set_whitespace(Whitespace::Space, Priority::Low);
    }
}

pub fn format_star(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    match state.mode {
        Mode::Code => format_padded(node, state, settings, output),
        _ => format_default(node, state, settings, output),
    }
}

pub fn format_underscore(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    match state.mode {
        Mode::Code => format_padded(node, state, settings, output),
        _ => format_default(node, state, settings, output),
    }
}
