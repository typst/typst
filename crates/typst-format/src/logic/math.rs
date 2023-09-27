use super::*;

pub fn format_equation(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let spaces = node
        .children()
        .fold(0, |spaces, child| spaces + (child.kind() == SyntaxKind::Space) as u32);
    if spaces < 2 || !equation_has_aligment(node) {
        format_inline_equation(node, state, settings, output, spaces == 2);
    } else {
        format_multi_line_equation(node, state, settings, output);
    }
}

pub fn format_multi_line_equation(
    node: &SyntaxNode,
    mut state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Dollar => match (state.mode, &settings.block.long_block_style) {
                (Mode::Math, LongBlockStyle::Compact) => {
                    output.set_whitespace(Whitespace::Space, Priority::Guaranteed);
                    output.raw(child, &state, settings);
                    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
                }
                (_, LongBlockStyle::Compact) => {
                    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
                    output.raw(child, &state, settings);
                    output.set_whitespace(Whitespace::Space, Priority::Normal);
                    state.mode = Mode::Math;
                }
                (Mode::Math, LongBlockStyle::Seperate) => {
                    state.dedent();
                    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
                    output.raw(child, &state, settings);
                }
                (_, LongBlockStyle::Seperate) => {
                    output.raw(child, &state, settings);
                    state.indent();
                    output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
                    state.mode = Mode::Math;
                }
            },
            SyntaxKind::Math => {
                format_multi_line_math(child, state, settings, output);
            }
            _ => format(child, state, settings, output),
        }
    }
}

fn format_multi_line_math(
    node: &SyntaxNode,
    mut state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    let has_align = node
        .children()
        .any(|child| child.kind() == SyntaxKind::MathAlignPoint);
    if !has_align {
        for child in node.children() {
            format(child, state, settings, output);
        }
        return;
    }
    let mut lengths = Vec::new();
    lengths.push(Vec::new());
    let mut calculator = PositionCalculator::new();
    let mut calc = Output::new(&mut calculator);
    for child in node.children() {
        match child.kind() {
            SyntaxKind::MathAlignPoint => {
                lengths.last_mut().unwrap().push(calc.position().1);
                calc.reset();
            }
            SyntaxKind::Linebreak => {
                lengths.last_mut().unwrap().push(calc.position().1);
                calc.reset();
                lengths.push(Vec::new());
            }
            _ => {
                format(child, state, settings, &mut calc);
            }
        }
    }
    if calc.position().1 != 0 {
        lengths.last_mut().unwrap().push(calc.position().1);
    }

    let columns_amount = lengths.iter().map(|v| v.len()).max().unwrap_or(1);
    let mut columns = vec![0; columns_amount];
    for l in lengths.iter() {
        for (index, &v) in l.iter().enumerate() {
            columns[index] = columns[index].max(v);
        }
    }
    let mut line = 0;
    let mut index = 0;
    if let LongBlockStyle::Compact = settings.block.long_block_style {
        state.extra_indentation = 2;
    }
    for child in node.children() {
        match child.kind() {
            SyntaxKind::MathAlignPoint => {
                output.set_whitespace(Whitespace::None, Priority::Normal);
                output.emit_whitespace(&state, settings);
                let diff = 1 + columns[index] - lengths[line][index];
                output.set_whitespace(Whitespace::Spaces(diff), Priority::Normal);
                format(child, state, settings, output);
                output.set_whitespace(Whitespace::Space, Priority::Normal);
                index += 1;
            }
            SyntaxKind::Linebreak => {
                output.set_whitespace(Whitespace::None, Priority::Normal);
                output.emit_whitespace(&state, settings);
                let amount = 1
                    + columns[index..].iter().sum::<usize>()
                    + (columns_amount - index - 1) * 3;
                let diff = amount - lengths[line][index];
                output.set_whitespace(Whitespace::Spaces(diff), Priority::High);
                format(child, state, settings, output);
                output.set_whitespace(Whitespace::LineBreak, Priority::High);
                line += 1;
                index = 0;
            }
            _ => {
                format(child, state, settings, output);
            }
        }
    }
}

fn format_inline_equation(
    node: &SyntaxNode,
    mut state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
    needs_spaces: bool,
) {
    for child in node.children() {
        match (state.mode, child.kind()) {
            (Mode::Math, SyntaxKind::Dollar) => {
                if needs_spaces {
                    output.set_whitespace(Whitespace::Space, Priority::Guaranteed);
                } else {
                    output.set_whitespace(Whitespace::None, Priority::High);
                }
                output.raw(child, &state, settings);
            }
            (_, SyntaxKind::Dollar) => {
                output.raw(child, &state, settings);
                if needs_spaces {
                    output.set_whitespace(Whitespace::Space, Priority::Guaranteed);
                } else {
                    output.set_whitespace(Whitespace::None, Priority::High);
                }
                state.indent();
                if let LongBlockStyle::Compact = settings.block.long_block_style {
                    state.extra_indentation = 2;
                }
                state.mode = Mode::Math;
            }
            _ => format(child, state, settings, output),
        }
    }
}

fn equation_has_aligment(node: &SyntaxNode) -> bool {
    for child in node.children() {
        if child.kind() != SyntaxKind::Math {
            continue;
        }
        for sub_child in child.children() {
            match sub_child.kind() {
                SyntaxKind::Linebreak => return true,
                SyntaxKind::MathAlignPoint => return true,
                _ => {}
            }
        }
    }
    false
}

pub fn format_math_attach(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::Underscore | SyntaxKind::Hat => {
                output.set_whitespace(Whitespace::None, Priority::High);
                format(child, state, settings, output);
                output.set_whitespace(Whitespace::None, Priority::High);
            }
            _ => format(child, state, settings, output),
        }
    }
}
