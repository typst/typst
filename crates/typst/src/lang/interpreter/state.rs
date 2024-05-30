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

/// The current state of the VM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    /// Whether to display the output.
    pub display: bool,

    /// Whether we are looping.
    pub looping: bool,

    /// The current flow event.
    pub flow: Flow,
}

impl State {
    /// Create a new state.
    pub fn empty() -> Self {
        Self { display: false, looping: false, flow: Flow::None }
    }

    /// Enable or disable displaying the output.
    pub fn display(mut self, display: bool) -> Self {
        self.display = display;
        self
    }

    /// Enable or disable looping.
    pub fn loop_(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Checks if we are currently displaying
    pub fn is_display(&self) -> bool {
        self.display
    }

    /// Checks if we are currently looping
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Set the current flow as being done evaluating.
    pub fn done(&mut self) {
        self.flow = Flow::Done;
    }

    /// Checks if the current flow is not done.
    pub fn is_running(&self) -> bool {
        !matches!(self.flow, Flow::Done)
    }

    /// Checks if the current flow is done.
    pub fn is_done(&self) -> bool {
        matches!(self.flow, Flow::Done)
    }

    /// Set the current flow as done.
    pub fn set_done(&mut self) {
        self.flow = Flow::Done;
    }

    /// Checks if the current flow is breaking.
    pub fn is_breaking(&self) -> bool {
        matches!(self.flow, Flow::Break)
    }

    /// Set the current flow as breaking.
    pub fn set_breaking(&mut self) {
        self.flow = Flow::Break;
    }

    /// Checks if the current flow is continuing.
    pub fn is_continuing(&self) -> bool {
        matches!(self.flow, Flow::Continue)
    }

    /// Set the current flow as continuing.
    pub fn set_continuing(&mut self) {
        self.flow = Flow::Continue;
    }

    /// Checks if the current flow is returning.
    pub fn is_returning(&self) -> bool {
        matches!(self.flow, Flow::Return(_))
    }

    /// Set the current flow as returning.
    pub fn set_returning(&mut self, forced: bool) {
        self.flow = Flow::Return(forced);
    }
}
