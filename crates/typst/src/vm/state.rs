use crate::foundations::Value;

#[derive(Debug)]
pub enum ControlFlow {
    Done(Value),
    Break(Value),
    Continue(Value),
    Return(Value, bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    None,
    Done,
    Break,
    Continue,
    Return(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub display: bool,
    pub looping: bool,
    pub flow: Flow,
}

impl State {
    pub fn empty() -> Self {
        Self { display: false, looping: false, flow: Flow::None }
    }

    pub fn display(mut self, display: bool) -> Self {
        self.display = display;
        self
    }

    pub fn loop_(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn done(&mut self) {
        self.flow = Flow::Done;
    }

    pub fn is_display(&self) -> bool {
        self.display
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    pub fn is_running(&self) -> bool {
        !matches!(self.flow, Flow::Done)
    }

    pub fn is_done(&self) -> bool {
        matches!(self.flow, Flow::Done)
    }

    pub fn set_done(&mut self) {
        self.flow = Flow::Done;
    }

    pub fn is_breaking(&self) -> bool {
        matches!(self.flow, Flow::Break)
    }

    pub fn set_breaking(&mut self) {
        self.flow = Flow::Break;
    }

    pub fn is_continuing(&self) -> bool {
        matches!(self.flow, Flow::Continue)
    }

    pub fn set_continuing(&mut self) {
        self.flow = Flow::Continue;
    }

    pub fn is_returning(&self) -> bool {
        matches!(self.flow, Flow::Return(_))
    }

    pub fn set_returning(&mut self, forced: bool) {
        self.flow = Flow::Return(forced);
    }
}
