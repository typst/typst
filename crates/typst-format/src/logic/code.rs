use super::*;

pub fn format_code_block(
    node: &SyntaxNode,
    mut state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let single = !node
        .children()
        .any(|value| value.kind() == SyntaxKind::Space && value.text().contains('\n'));
    state.mode = Mode::Code;
    for child in node.children() {
        match child.kind() {
            SyntaxKind::LeftBrace => {
                format(child, state, settings, output);
                if single {
                    output.set_whitespace(Whitespace::Space, Priority::Low);
                } else {
                    state.indent();
                    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
                }
            }
            SyntaxKind::Code => format(child, state, settings, output),
            SyntaxKind::RightBrace => {
                if single {
                    output.set_whitespace(Whitespace::Space, Priority::Low);
                } else {
                    state.dedent();
                    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
                }
                format(child, state, settings, output);
            }
            _ => format(child, state, settings, output),
        }
    }
}

pub fn format_func_call(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    enum Kind {
        Normal,
        Columns,
    }
    let mut kind = Kind::Normal;
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Ident => {
                kind = match child.text().as_str() {
                    "table" => Kind::Columns,
                    "grid" => Kind::Columns,
                    _ => Kind::Normal,
                };
                format(child, state, settings, output);
            }
            SyntaxKind::Args => match kind {
                Kind::Normal => format_items(child, state, settings, output),
                Kind::Columns => format_column_args(child, state, settings, output),
            },
            _ => format(child, state, settings, output),
        }
    }
}

pub fn format_unary(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Plus | SyntaxKind::Minus => {
                format(child, state, settings, output);
                output.set_whitespace(Whitespace::None, Priority::High);
            }
            _ => format(child, state, settings, output),
        }
    }
}

pub fn format_named_argument(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Colon => {
                if settings.named_argument.space_before_colon {
                    output.set_whitespace(Whitespace::Space, Priority::High);
                } else {
                    output.set_whitespace(Whitespace::None, Priority::High);
                }
                format(child, state, settings, output);
                if settings.named_argument.space_after_colon {
                    output.set_whitespace(Whitespace::Space, Priority::High);
                } else {
                    output.set_whitespace(Whitespace::None, Priority::High);
                }
            }
            _ => format(child, state, settings, output),
        }
    }
}

pub fn format_keyed(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Colon => {
                if settings.dictionary_entry.space_before_colon {
                    output.set_whitespace(Whitespace::Space, Priority::High);
                } else {
                    output.set_whitespace(Whitespace::None, Priority::High);
                }
                format(child, state, settings, output);
                if settings.dictionary_entry.space_after_colon {
                    output.set_whitespace(Whitespace::Space, Priority::High);
                } else {
                    output.set_whitespace(Whitespace::None, Priority::High);
                }
            }
            _ => format(child, state, settings, output),
        }
    }
}

pub fn format_semicolon(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    output.set_whitespace(Whitespace::None, Priority::Guaranteed);
    output.raw(node, &state, settings);
}

pub fn format_items(
    node: &SyntaxNode,
    mut state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let mut trailing_comma = false;
    let mut comma_count = 0;
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Comma => (trailing_comma, comma_count) = (true, comma_count + 1),
            SyntaxKind::RightParen => break,
            SyntaxKind::Space => {}
            _ => trailing_comma = false,
        }
    }
    state.mode = Mode::Items;
    let single = !trailing_comma || comma_count <= 1;
    for child in node.children() {
        match child.kind() {
            SyntaxKind::LeftParen => {
                format(child, state, settings, output);
                if single {
                    output.set_whitespace(Whitespace::None, Priority::Guaranteed);
                } else {
                    state.indent();
                    output.set_whitespace(Whitespace::LineBreak, Priority::High);
                }
            }
            SyntaxKind::Comma => {
                format(child, state, settings, output);
                if single {
                    output.set_whitespace(Whitespace::Space, Priority::Low);
                } else {
                    output.set_whitespace(Whitespace::LineBreak, Priority::High);
                }
            }
            SyntaxKind::RightParen => {
                if single {
                    output.set_whitespace(Whitespace::None, Priority::High);
                } else {
                    state.dedent();
                    output.set_whitespace(Whitespace::LineBreak, Priority::High);
                }
                format(child, state, settings, output);
            }
            _ => format(child, state, settings, output),
        }
    }
}

pub fn format_column_args(
    node: &SyntaxNode,
    mut state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let columns_count = get_column_count(node);
    let mut lengths = Vec::new();
    for child in node.children() {
        match child.kind() {
            SyntaxKind::LeftParen
            | SyntaxKind::RightParen
            | SyntaxKind::Comma
            | SyntaxKind::Space
            | SyntaxKind::LineComment
            | SyntaxKind::BlockComment
            | SyntaxKind::Named => {
                continue;
            }
            _ => {}
        }
        lengths.push(get_length(child, settings));
    }
    let mut columns: Vec<usize> = vec![1usize; columns_count];
    for (index, &lenght) in lengths.iter().enumerate() {
        let c = index % columns_count;
        columns[c] = columns[c].max(lenght.unwrap_or(0));
    }

    state.mode = Mode::Items;

    let mut current = 0usize;
    let mut index = 0;
    let mut pad = false;
    for child in node.children() {
        match child.kind() {
            SyntaxKind::LeftParen => {
                format(child, state, settings, output);
                state.indent();
                output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
            }
            SyntaxKind::Comma => {
                if pad {
                    match settings.columns.comma {
                        AlignComma::EndOfContent => {
                            output.set_whitespace(Whitespace::None, Priority::High);
                            format(child, state, settings, output);
                            if let Some(value) = lengths[index] {
                                output.set_whitespace(
                                    Whitespace::Spaces(columns[current] - value + 1),
                                    Priority::Normal,
                                );
                            }
                        }
                        AlignComma::EndOfCell => {
                            if let Some(value) = lengths[index] {
                                output.set_whitespace(
                                    Whitespace::Spaces(columns[current] - value),
                                    Priority::High,
                                );
                            }
                            format(child, state, settings, output);
                        }
                    }
                    current = (current + 1) % columns_count;
                    index += 1;
                    if current == 0 {
                        output.set_whitespace(Whitespace::LineBreak, Priority::High);
                    } else {
                        output.set_whitespace(Whitespace::Space, Priority::Low);
                    }
                    pad = false;
                } else {
                    format(child, state, settings, output);
                    output.set_whitespace(Whitespace::LineBreak, Priority::High);
                }
            }
            SyntaxKind::RightParen => {
                state.dedent();
                output.set_whitespace(Whitespace::LineBreak, Priority::High);
                format(child, state, settings, output);
            }
            SyntaxKind::Named => {
                format(child, state, settings, output);
                pad = false;
            }
            _ => {
                format(child, state, settings, output);
                pad = true;
            }
        }
    }
}

fn get_column_count(node: &SyntaxNode) -> usize {
    for child in node.children() {
        if child.kind() != SyntaxKind::Named {
            continue;
        }
        enum State {
            Start,
            IsColumns,
            Columns(usize),
        }

        let state = child.children().fold(State::Start, |state, sub_child| {
            match (&state, sub_child.kind()) {
                (State::Start, SyntaxKind::Ident) => {
                    if sub_child.text() == "columns" {
                        State::IsColumns
                    } else {
                        State::Start
                    }
                }
                (State::IsColumns, SyntaxKind::Array) => {
                    let count =
                        sub_child.children().fold(0, |count, value| match value.kind() {
                            SyntaxKind::Auto
                            | SyntaxKind::Int // would be compile error, but would be strange to skip for formatting
                            | SyntaxKind::Numeric
                            | SyntaxKind::Float => count + 1,
                            _ => count,
                        });
                    State::Columns(count)
                }
                (State::IsColumns, SyntaxKind::Int) => {
                    State::Columns(sub_child.text().parse().unwrap_or(1))
                }
                _ => state,
            }
        });
        if let State::Columns(value) = state {
            return if value == 0 { 1 } else { value };
        }
    }
    1
}

pub fn format_code_statement(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    format_default(node, state, settings, output);
    match state.mode {
        Mode::Code => {}
        _ => output.set_whitespace(Whitespace::LineBreak, Priority::Normal),
    }
}
