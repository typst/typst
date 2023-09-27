use ecow::EcoString;
use typst_syntax::SyntaxNode;

use crate::state::State;

use super::settings::Settings;

#[derive(Clone, Copy)]
pub enum Whitespace {
    None,
    Space,
    Spaces(usize),
    LineBreak,
    LineBreaks(usize),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Priority {
    Lowest,
    Low,
    Normal,
    High,
    Guaranteed,
}

pub trait OutputTarget {
    fn emit(&mut self, data: &EcoString, settings: &Settings);
}

pub struct Output<'a, Target: OutputTarget> {
    target: &'a mut Target,
    whitespace: Whitespace,
    priority: Priority,
}

impl<'a, Target: OutputTarget> Output<'a, Target> {
    pub fn new(target: &'a mut Target) -> Self {
        Self {
            target,
            whitespace: Whitespace::None,
            priority: Priority::Guaranteed,
        }
    }

    fn emit_indentation(&mut self, state: &State, settings: &Settings) {
        if state.indentation + state.extra_indentation == 0 {
            return;
        }
        match settings.indentation {
            0 => {
                let mut data =
                    EcoString::with_capacity(state.indentation + state.extra_indentation);
                for _ in 0..state.indentation {
                    data.push('\t');
                }
                for _ in 0..state.extra_indentation {
                    data.push(' ');
                }
                self.target.emit(&data, settings);
            }
            amount => {
                let length = state.indentation * amount;
                let mut data = EcoString::with_capacity(length + state.extra_indentation);
                for _ in 0..length {
                    data.push(' ');
                }
                for _ in 0..state.extra_indentation {
                    data.push(' ');
                }
                self.target.emit(&data, settings);
            }
        }
    }

    pub fn emit_whitespace(&mut self, state: &State, settings: &Settings) {
        match self.whitespace {
            Whitespace::None => {}
            Whitespace::Space => self.target.emit(&EcoString::inline(" "), settings),
            Whitespace::Spaces(amount) => {
                let mut data = EcoString::with_capacity(amount);
                for _ in 0..amount {
                    data.push(' ');
                }
                self.target.emit(&data, settings);
            }
            Whitespace::LineBreak => {
                self.target.emit(&EcoString::inline("\n"), settings);
                self.emit_indentation(state, settings)
            }
            Whitespace::LineBreaks(amount) => {
                let mut data = EcoString::with_capacity(amount);
                for _ in 0..amount {
                    data.push('\n');
                }
                self.target.emit(&data, settings);
                self.emit_indentation(state, settings)
            }
        }
        self.whitespace = Whitespace::None;
        self.priority = Priority::Lowest;
    }

    pub fn raw(&mut self, node: &SyntaxNode, state: &State, settings: &Settings) {
        if node.text().is_empty() {
            return;
        }
        self.emit_whitespace(state, settings);
        self.target.emit(node.text(), settings);
    }

    pub fn set_whitespace(&mut self, whitespace: Whitespace, priority: Priority) {
        if self.priority == priority {
            // use larger whitespace
            match (self.whitespace, whitespace) {
                (Whitespace::None, _) => {}
                (Whitespace::Space, Whitespace::Spaces(_)) => {}
                (Whitespace::Space, Whitespace::LineBreak) => {}
                (Whitespace::Space, Whitespace::LineBreaks(_)) => {}
                (Whitespace::Spaces(before), Whitespace::Spaces(after))
                    if after > before => {}
                (Whitespace::Spaces(_), Whitespace::LineBreak) => {}
                (Whitespace::Spaces(_), Whitespace::LineBreaks(_)) => {}
                (Whitespace::LineBreak, Whitespace::LineBreaks(_)) => {}
                (Whitespace::LineBreaks(before), Whitespace::LineBreaks(after))
                    if after > before => {}
                _ => return,
            }
        } else {
            // use higher priority
            match (self.priority, priority) {
                (Priority::Lowest, _) => {}
                (Priority::Low, Priority::Normal) => {}
                (Priority::Low, Priority::High) => {}
                (Priority::Normal, Priority::High) => {}
                (_, Priority::Guaranteed) => {}
                _ => return,
            }
        }
        self.whitespace = whitespace;
        self.priority = priority;
    }

    pub fn get_whitespace(&self) -> (Whitespace, Priority) {
        (self.whitespace, self.priority)
    }

    pub fn finish(mut self, state: &State, settings: &Settings) {
        self.emit_whitespace(state, settings);
    }
}

pub struct PositionCalculator {
    line: usize,
    column: usize,
}

impl PositionCalculator {
    pub fn new() -> Self {
        Self { line: 0, column: 0 }
    }

    pub fn reset(&mut self) {
        self.line = 0;
        self.column = 0;
    }
}

impl OutputTarget for PositionCalculator {
    fn emit(&mut self, data: &EcoString, settings: &Settings) {
        for symbol in data.chars() {
            match symbol {
                '\t' => {
                    let tab_size = settings.indentation.max(1);
                    self.column += 1 + tab_size.overflowing_sub(self.column).0 % tab_size
                }
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                }
                _ => self.column += 1,
            }
        }
    }
}

impl Output<'_, PositionCalculator> {
    pub fn position(&self) -> (usize, usize) {
        (self.target.line, self.target.column)
    }

    pub fn reset(&mut self) {
        self.target.reset();
        self.whitespace = Whitespace::None;
        self.priority = Priority::Guaranteed;
    }
}
