use ecow::EcoVec;

use crate::layout::Regions;

use super::{Work, ColumnConfig, collect::Child};

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

pub(in super) struct Balancer<'a, 'b> {
    /// All children to be balanced.
    children: &'b [Child<'a>],
    /// Current work state.
    work: Work<'a, 'b>,
    /// Information about balanced columns.
    config: &'a ColumnConfig,
    /// Index of the current column.
    current_column: usize,
    /// Mode of balancing.
    mode: Mode,
}

impl<'a, 'b> Balancer<'a, 'b> {
    pub fn new(children: &'b [Child<'a>], config: &'a ColumnConfig, mode: Mode) -> Self {
        Self {
            children,
            work: Work::new(&[]),
            config,
            current_column: 0,
            mode,
        }
    }

    pub fn borrow_work(&mut self, regions: Regions) -> WorkLimiter<'a, 'b, '_> {
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

pub(in super) struct WorkLimiter<'a, 'b, 'c> {
    owner: &'c mut Balancer<'a, 'b>,
    advance: usize,
}
impl<'a, 'b, 'c> std::ops::Deref for WorkLimiter<'a, 'b, 'c> {
    type Target = Work<'a, 'b>;

    fn deref(&self) -> &Self::Target {
        &self.owner.work
    }
}
impl<'a, 'b, 'c> std::ops::DerefMut for WorkLimiter<'a, 'b, 'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.owner.work
    }
}
impl<'a, 'b, 'c> Drop for WorkLimiter<'a, 'b, 'c> {
    fn drop(&mut self) {
        let remaining = self.owner.work.children.len();
        let skip = self.advance - remaining;
        self.owner.children = &self.owner.children[skip..];
        self.owner.current_column += 1;
    }
}
