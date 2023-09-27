#[derive(Clone, Copy)]
pub enum Mode {
    /// `[...]` or top level
    Markdown,
    /// `{...}`
    Code,
    /// `$...$`
    Math,
    /// `(_, ...)`
    Items,
}

#[derive(Clone, Copy)]
pub struct State {
    pub indentation: usize,
    pub extra_indentation: usize,
    pub mode: Mode,
}

impl State {
    pub fn new() -> Self {
        Self {
            indentation: 0,
            extra_indentation: 0,
            mode: Mode::Markdown,
        }
    }
    pub fn indent(&mut self) {
        self.indentation += 1;
    }

    pub fn dedent(&mut self) {
        self.indentation -= 1
    }
}
