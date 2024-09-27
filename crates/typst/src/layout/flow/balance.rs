use ecow::EcoVec;

use crate::{layout::{Regions, Size}, engine::Engine, introspection::Locator};

use super::{Work, collect::Child, Config as FlowConfig};

/// Balancing mode.
/// 
/// Dictates how the content will be balanced across available columns.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Eagerly place items from first to last column.
    PackStart,
    /// Attempt equally balancing items across available columns.
    Even,
}

trait BalancerStage<'a, 'b> {
    type Children;
}

struct Initialized;
impl<'a, 'b> BalancerStage<'a, 'b> for Initialized where 'a: 'b {
    type Children = &'b [Child<'a>];
}

struct Measured;
impl<'a, 'b> BalancerStage<'a, 'b> for Measured where 'a: 'b {
    type Children = &'b [Child<'a>];
}

pub(in super) struct Balancer<'a, 'b, 'x, S> where S: BalancerStage<'a, 'b> {
    /// All children to be balanced.
    children: S::Children,
    /// Current work state.
    work: Work<'a, 'b>,
    /// Information about balanced columns.
    config: &'a FlowConfig<'x>,
    /// Index of the current column.
    current_column: usize,
    /// Mode of balancing.
    mode: Mode,
}

impl<'a, 'b, 'x> Balancer<'a, 'b, 'x, Initialized> {
    pub fn new(children: &'b [Child<'a>], config: &'a FlowConfig<'x>, mode: Mode) -> Self {
        Self {
            children,
            work: Work::new(&[]),
            config,
            current_column: 0,
            mode,
        }
    }

    pub fn measure(self,
        engine: &mut Engine,
        work: &mut Work,
        locator: &Locator,
    ) -> Balancer<'a, 'b, 'x, Measured> {
        let locator = locator.relayout();

        Balancer {
            children: self.children,
            work: self.work,
            config: self.config,
            current_column: 0,
            mode: self.mode,
        }
    }
}

impl<'a, 'b, 'x> Balancer<'a, 'b, 'x, Measured> {
    pub fn borrow_work(&mut self, regions: Regions) -> WorkLimiter<'a, 'b, 'x, '_> {
        self.work.children = match self.mode {
            Mode::PackStart => self.children,
            Mode::Even => &self.children[..1], // FIXME: Measure how many fit
        };
        let advance = self.work.children.len();
        WorkLimiter {
            owner: self,
            advance,
        }
    }
}

pub(in super) struct WorkLimiter<'a, 'b, 'x, 'c> {
    owner: &'c mut Balancer<'a, 'b, 'x, Measured>,
    advance: usize,
}
impl<'a, 'b, 'x, 'c> std::ops::Deref for WorkLimiter<'a, 'b, 'x, 'c> {
    type Target = Work<'a, 'b>;

    fn deref(&self) -> &Self::Target {
        &self.owner.work
    }
}
impl<'a, 'b, 'x, 'c> std::ops::DerefMut for WorkLimiter<'a, 'b, 'x, 'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.owner.work
    }
}
impl<'a, 'b, 'x, 'c> Drop for WorkLimiter<'a, 'b, 'x, 'c> {
    fn drop(&mut self) {
        let remaining = self.owner.work.children.len();
        let skip = self.advance - remaining;
        self.owner.children = &self.owner.children[skip..];
        self.owner.current_column += 1;
    }
}
