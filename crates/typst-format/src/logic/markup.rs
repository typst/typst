use super::*;

pub fn format_markup(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let mut disabled = false;
    for child in node.children() {
        if (child.kind() == SyntaxKind::LineComment
            || child.kind() == SyntaxKind::BlockComment)
            && child.text().contains("typstfmt")
        {
            if child.text().contains("disable") {
                disabled = true;
            } else if child.text().contains("enable") {
                disabled = false;
            }
        }
        if disabled {
            skip_formatting(child, state, settings, output);
        } else {
            format(child, state, settings, output);
        }
    }
}

pub fn format_content_block(
    node: &SyntaxNode,
    mut state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let mut start_space = false;
    let mut end_space = false;
    let mut linebreak = false;
    for child in node.children() {
        if child.kind() != SyntaxKind::Markup {
            continue;
        }
        for (index, sub_child) in child.children().enumerate() {
            match (index, sub_child.kind()) {
                (0, SyntaxKind::Space) | (0, SyntaxKind::Parbreak) => {
                    start_space = true;
                    end_space = true;
                    if sub_child.text().contains('\n') {
                        linebreak = true;
                    }
                }
                (_, SyntaxKind::Space) | (_, SyntaxKind::Parbreak) => {
                    end_space = true;
                    if sub_child.text().contains('\n') {
                        linebreak = true;
                    }
                }
                _ => end_space = false,
            }
        }
    }
    let single = !start_space || !end_space || !linebreak;
    state.mode = Mode::Markdown;
    for child in node.children() {
        match child.kind() {
            SyntaxKind::LeftBracket => {
                format(child, state, settings, output);
                if single {
                    if start_space {
                        output.set_whitespace(Whitespace::Space, Priority::Guaranteed);
                    } else {
                        output.set_whitespace(Whitespace::None, Priority::Guaranteed);
                    }
                } else {
                    state.indent();
                    match settings.block.long_block_style {
                        LongBlockStyle::Compact => {
                            output.set_whitespace(Whitespace::Space, Priority::Low)
                        }
                        LongBlockStyle::Seperate => {
                            output.set_whitespace(Whitespace::LineBreak, Priority::Normal)
                        }
                    }
                }
            }

            SyntaxKind::RightBracket => {
                if single {
                    if end_space {
                        output.set_whitespace(Whitespace::Space, Priority::Guaranteed);
                    } else {
                        output.set_whitespace(Whitespace::None, Priority::Guaranteed);
                    }
                } else {
                    state.dedent();
                    match settings.block.long_block_style {
                        LongBlockStyle::Compact => {
                            output.set_whitespace(Whitespace::Space, Priority::Low)
                        }
                        LongBlockStyle::Seperate => {
                            output.set_whitespace(Whitespace::LineBreak, Priority::Normal)
                        }
                    }
                }
                format(child, state, settings, output);
            }
            _ => format(child, state, settings, output),
        }
    }
}

pub fn format_heading(
    settings: &Settings,
    node: &SyntaxNode,
    state: State,
    output: &mut Output<impl OutputTarget>,
) {
    output.set_whitespace(
        Whitespace::LineBreaks(settings.heading.blank_lines_before + 1),
        Priority::High,
    );
    format_default(node, state, settings, output);
    output.set_whitespace(
        Whitespace::LineBreaks(settings.heading.blank_lines_after + 1),
        Priority::High,
    );
}

pub fn format_label(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let (whitespace, priority) = output.get_whitespace();
    if settings.seperate_label {
        output.set_whitespace(Whitespace::Space, Priority::Guaranteed);
    } else {
        output.set_whitespace(Whitespace::None, Priority::High);
    }
    output.raw(node, &state, settings);
    output.set_whitespace(whitespace, priority);
}

pub fn format_term(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Colon => {
                if settings.term.space_before_colon {
                    output.set_whitespace(Whitespace::Space, Priority::Normal);
                } else {
                    output.set_whitespace(Whitespace::None, Priority::Normal);
                }
                format(child, state, settings, output);
                if settings.term.space_after_colon {
                    output.set_whitespace(Whitespace::Space, Priority::Normal);
                } else {
                    output.set_whitespace(Whitespace::None, Priority::Normal);
                }
            }
            _ => format(child, state, settings, output),
        }
    }
    output.set_whitespace(Whitespace::LineBreak, Priority::High);
}

pub fn format_end_of_file(
    _node: &SyntaxNode,
    _state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    if settings.final_newline {
        output.set_whitespace(Whitespace::LineBreak, Priority::Guaranteed);
    } else {
        output.set_whitespace(Whitespace::None, Priority::Guaranteed);
    }
}
