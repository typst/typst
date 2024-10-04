use std::collections::HashMap;

use crate::{
    diag::SourceResult,
    engine::Engine,
    introspection::Locator,
    layout::{Abs, Axes, Region, Regions, Size},
};

use super::{collect::Child, compose, Config as FlowConfig, Work};

/// Balancing mode.
///
/// Dictates how the content will be balanced across available columns.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Mode {
    /// Eagerly place items from first to last column.
    PackStart,
    /// Attempt equally balancing items across available columns.
    Even,
}

trait BalancerStage<'a, 'b>: Sized {
    type Data;

    fn mode(balancer: &Balancer<'a, 'b, '_, Self>) -> Mode;
}

struct Initialized;
impl<'a, 'b> BalancerStage<'a, 'b> for Initialized
where
    'a: 'b,
{
    type Data = Mode;

    #[inline]
    fn mode(balancer: &Balancer<'a, 'b, '_, Self>) -> Mode {
        balancer.metadata
    }
}

struct Measured;
impl<'a, 'b> BalancerStage<'a, 'b> for Measured
where
    'a: 'b,
{
    type Data = ChildrenMetadata;

    #[inline]
    fn mode(balancer: &Balancer<'a, 'b, '_, Self>) -> Mode {
        balancer.metadata.mode()
    }
}

#[repr(u8, C)]
enum ChildrenMetadata {
    PackStart = Mode::PackStart as u8,
    Even {
        ideal_height: Abs,
        children_size: HashMap<usize, Axes<Abs>>,
    } = Mode::Even as u8,
}

impl ChildrenMetadata {
    #[inline]
    fn mode(&self) -> Mode {
        unsafe {
            // SAFETY: Docs say pointer cast of #[repr(u8)] enum to u8
            // discriminant value is safe.
            *(self as *const Self as *const Mode).as_ref().unwrap_unchecked()
        }
    }
}

pub(super) struct Balancer<'a, 'b, 'x, S>
where
    S: BalancerStage<'a, 'b>,
{
    /// All children to be balanced.
    children: &'b [Child<'a>],
    /// Balancing information.
    metadata: S::Data,
    /// Current work state.
    work: Work<'a, 'b>,
    /// Information about balanced columns.
    config: &'a FlowConfig<'x>,
    /// Index of the current column.
    current_column: usize,
}

impl<'a, 'b, 'x, S> Balancer<'a, 'b, 'x, S>
where
    S: BalancerStage<'a, 'b>,
{
    #[inline]
    fn mode(&self) -> Mode {
        S::mode(self)
    }
}

impl<'a, 'b, 'x> Balancer<'a, 'b, 'x, Initialized> {
    pub fn new(
        children: &'b [Child<'a>],
        config: &'a FlowConfig<'x>,
        mode: Mode,
    ) -> Self {
        Self {
            children,
            metadata: mode,
            work: Work::new(&[]),
            config,
            current_column: 0,
        }
    }

    fn compute_even_child_columns(
        &self,
        engine: &mut Engine,
        locator: Locator,
        bounds: Region,
    ) -> SourceResult<ChildrenMetadata> {
        let regions = Regions::from(bounds);
        let mut work = Work::new(&self.children);
        let frame = compose(engine, &mut work, self.config, locator.relayout(), regions)?;

        let total_height = frame.height();
        let ideal_height =
            Abs::raw(total_height.to_raw() / self.config.columns.count as f64);

        let mut children_size = HashMap::new();

        let mut children = self.children.iter().enumerate();
        let mut frame_items = frame.items().map(|it| it.1);
        'outer: while let Some((i, child)) = children.next() {
            let size = loop {
                let item = match frame_items.next() {
                    Some(it) => it,
                    None => break 'outer,
                };

                // TODO: case where frame_items doesn't contain child will drain
                // frame_items and leave all children unmeasured. This inner
                // loop must never break to 'outer, i.e. every child must be in frame_items.

                if true
                /* FIXME: item == child */
                {
                    break item.size();
                }
            };
            let size = size.unwrap_or_default();
            children_size.insert(i, size);
        }

        Ok(ChildrenMetadata::Even { ideal_height, children_size })
    }

    pub fn measure(
        self,
        engine: &mut Engine,
        locator: Locator,
        bounds: Region,
    ) -> SourceResult<Balancer<'a, 'b, 'x, Measured>> {
        let metadata = match self.mode() {
            Mode::PackStart => ChildrenMetadata::PackStart,
            Mode::Even => self.compute_even_child_columns(engine, locator, bounds)?,
        };

        Ok(Balancer {
            children: self.children,
            metadata,
            work: self.work,
            config: self.config,
            current_column: 0,
        })
    }
}

impl<'a, 'b, 'x> Balancer<'a, 'b, 'x, Measured> {
    pub fn borrow_work(&mut self, regions: Regions) -> WorkLimiter<'a, 'b, 'x, '_> {
        self.work.children = match self.mode() {
            Mode::PackStart => self.children,
            Mode::Even => &self.children[..1], // FIXME: Measure how many fit
        };
        let advance = self.work.children.len();
        WorkLimiter { owner: self, advance }
    }
}

pub(super) struct WorkLimiter<'a, 'b, 'x, 'c> {
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
        // TODO: don't advance all children, use offset
        // self.owner.children = &self.owner.children[skip..];
        self.owner.current_column += 1;
    }
}
