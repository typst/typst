use std::fmt::{self, Debug, Formatter};

use crate::layout::{Abs, Axes, Size};

/// A single region to layout into.
#[derive(Debug, Copy, Clone, Hash)]
pub struct Region {
    /// The size of the region.
    pub size: Size,
    /// Whether elements should expand to fill the regions instead of shrinking
    /// to fit the content.
    pub expand: Axes<bool>,
}

impl Region {
    /// Create a new region.
    pub fn new(size: Size, expand: Axes<bool>) -> Self {
        Self { size, expand }
    }
}

impl From<Region> for Regions<'_> {
    fn from(region: Region) -> Self {
        Regions {
            size: region.size,
            expand: region.expand,
            full: region.size.y,
            backlog: &[],
            last: None,
        }
    }
}

/// A sequence of regions to layout into.
///
/// A *region* is a contiguous rectangular space in which elements
/// can be laid out. All regions within a `Regions` object have the
/// same width, namely `self.size.x`. This means that it is not
/// currently possible to, for instance, have content wrap to the
/// side of a floating element.
#[derive(Copy, Clone, Hash)]
pub struct Regions<'a> {
    /// The remaining size of the first region.
    pub size: Size,
    /// Whether elements should expand to fill the regions instead of shrinking
    /// to fit the content.
    pub expand: Axes<bool>,
    /// The full height of the region for relative sizing.
    pub full: Abs,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: &'a [Abs],
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Abs>,
}

impl Regions<'_> {
    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, expand: Axes<bool>) -> Self {
        Self {
            size,
            full: size.y,
            backlog: &[],
            last: Some(size.y),
            expand,
        }
    }

    /// The base size, which doesn't take into account that the regions is
    /// already partially used up.
    ///
    /// This is also used for relative sizing.
    pub fn base(&self) -> Size {
        Size::new(self.size.x, self.full)
    }

    /// Create new regions where all sizes are mapped with `f`.
    ///
    /// Note that since all regions must have the same width, the width returned
    /// by `f` is ignored for the backlog and the final region.
    pub fn map<'v, F>(&self, backlog: &'v mut Vec<Abs>, mut f: F) -> Regions<'v>
    where
        F: FnMut(Size) -> Size,
    {
        let x = self.size.x;
        backlog.clear();
        backlog.extend(self.backlog.iter().map(|&y| f(Size::new(x, y)).y));
        Regions {
            size: f(self.size),
            full: f(Size::new(x, self.full)).y,
            backlog,
            last: self.last.map(|y| f(Size::new(x, y)).y),
            expand: self.expand,
        }
    }

    /// Whether the first region is full and a region break is called for.
    pub fn is_full(&self) -> bool {
        Abs::zero().fits(self.size.y) && self.may_progress()
    }

    /// Whether a region break is permitted.
    pub fn may_break(&self) -> bool {
        !self.backlog.is_empty() || self.last.is_some()
    }

    /// Whether calling `next()` may improve a situation where there is a lack
    /// of space.
    pub fn may_progress(&self) -> bool {
        !self.backlog.is_empty() || self.last.is_some_and(|height| self.size.y != height)
    }

    /// Advance to the next region if there is any.
    pub fn next(&mut self) {
        if let Some(height) = self
            .backlog
            .split_first()
            .map(|(first, tail)| {
                self.backlog = tail;
                *first
            })
            .or(self.last)
        {
            self.size.y = height;
            self.full = height;
        }
    }

    /// An iterator that returns the sizes of the first and all following
    /// regions, equivalently to what would be produced by calling
    /// [`next()`](Self::next) repeatedly until all regions are exhausted.
    /// This iterator may be infinite.
    pub fn iter(&self) -> impl Iterator<Item = Size> + '_ {
        let first = std::iter::once(self.size);
        let backlog = self.backlog.iter();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&h| Size::new(self.size.x, h)))
    }
}

impl Debug for Regions<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Regions ")?;
        let mut list = f.debug_list();
        let mut prev = self.size.y;
        list.entry(&self.size);
        for &height in self.backlog {
            list.entry(&Size::new(self.size.x, height));
            prev = height;
        }
        if let Some(last) = self.last {
            if last != prev {
                list.entry(&Size::new(self.size.x, last));
            }
            list.entry(&(..));
        }
        list.finish()
    }
}

/// Width exclusions for text wrapping around floats.
///
/// Coordinates are relative to the paragraph's top-left corner.
/// Use `from_wrap_floats()` to convert from region coordinates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ParExclusions {
    /// Exclusion zones sorted by y_start.
    pub zones: Vec<ExclusionZone>,
}

/// A single rectangular exclusion zone.
///
/// Uses raw i64 units (not Abs) to avoid floating-point comparison issues
/// in sorted lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExclusionZone {
    /// Y-offset from paragraph top where exclusion starts (in raw units).
    pub y_start: i64,
    /// Y-offset from paragraph top where exclusion ends (in raw units).
    pub y_end: i64,
    /// Width excluded from left side (in raw units).
    pub left: i64,
    /// Width excluded from right side (in raw units).
    pub right: i64,
}

/// A positioned wrap-float in region coordinates.
///
/// Stores the position and dimensions of a float that text should wrap around.
/// The coordinates are in inner-flow (region) coordinate space, where y=0
/// is at the top of the content region.
#[derive(Debug, Clone)]
pub struct WrapFloat {
    /// Top y-coordinate in region (inner-flow) coordinates.
    pub y: Abs,
    /// Height of the float.
    pub height: Abs,
    /// Width excluded from left (float width + clearance, or zero if right-aligned).
    pub left_margin: Abs,
    /// Width excluded from right (float width + clearance, or zero if left-aligned).
    pub right_margin: Abs,
}

impl ParExclusions {
    /// Check if there are no exclusions.
    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    /// Create exclusions from wrap-float positions.
    ///
    /// Converts region-relative float coordinates to paragraph-relative
    /// exclusion zones. Only includes floats that overlap the paragraph's
    /// vertical extent.
    ///
    /// # Arguments
    /// * `par_y` - Paragraph's y-position in region coordinates
    /// * `par_height` - Estimated paragraph height (for overlap detection)
    /// * `wrap_floats` - List of positioned wrap-floats in region coordinates
    pub fn from_wrap_floats(
        par_y: Abs,
        par_height: Abs,
        wrap_floats: &[WrapFloat],
    ) -> Self {
        let mut zones = Vec::with_capacity(wrap_floats.len());
        let par_top = par_y.to_raw();
        let par_bottom = (par_y + par_height).to_raw();

        for wf in wrap_floats {
            let wf_top = wf.y.to_raw();
            let wf_bottom = (wf.y + wf.height).to_raw();

            // Skip floats that don't overlap this paragraph
            if wf_bottom <= par_top || wf_top >= par_bottom {
                continue;
            }

            // Convert to paragraph-relative coordinates, clamped to paragraph bounds
            let rel_start = (wf_top - par_top).max(0.0);
            let rel_end = (wf_bottom - par_top).min(par_bottom - par_top);

            zones.push(ExclusionZone {
                y_start: rel_start as i64,
                y_end: rel_end as i64,
                left: wf.left_margin.to_raw() as i64,
                right: wf.right_margin.to_raw() as i64,
            });
        }

        // Sort by y_start for efficient lookup
        zones.sort_by_key(|z| z.y_start);

        Self { zones }
    }

    /// Get available width at a given y-offset within the paragraph.
    ///
    /// If multiple exclusions overlap at this y, takes the maximum from each side.
    pub fn available_width(&self, base_width: Abs, y: Abs) -> Abs {
        let y_raw = y.to_raw() as i64;
        let mut left_excluded = 0i64;
        let mut right_excluded = 0i64;

        for zone in &self.zones {
            // Early exit: zones are sorted, so if y < y_start, no more matches
            if y_raw < zone.y_start {
                break;
            }
            if y_raw < zone.y_end {
                left_excluded = left_excluded.max(zone.left);
                right_excluded = right_excluded.max(zone.right);
            }
        }

        let total_excluded = Abs::raw(left_excluded as f64) + Abs::raw(right_excluded as f64);
        (base_width - total_excluded).max(Abs::zero())
    }

    /// Get left offset at a given y-offset (for text positioning).
    pub fn left_offset(&self, y: Abs) -> Abs {
        let y_raw = y.to_raw() as i64;
        let mut left = 0i64;

        for zone in &self.zones {
            if y_raw < zone.y_start {
                break;
            }
            if y_raw < zone.y_end {
                left = left.max(zone.left);
            }
        }

        Abs::raw(left as f64)
    }

    /// Check if any exclusion is active at this y-offset.
    pub fn has_exclusion_at(&self, y: Abs) -> bool {
        let y_raw = y.to_raw() as i64;
        self.zones.iter().any(|z| y_raw >= z.y_start && y_raw < z.y_end)
    }

    /// Get the next y-position where exclusions change (for line breaking).
    ///
    /// Returns the next y where an exclusion starts or ends, enabling the
    /// line-breaking algorithm to skip to known boundary points.
    pub fn next_boundary(&self, y: Abs) -> Option<Abs> {
        let y_raw = y.to_raw() as i64;
        self.zones
            .iter()
            .flat_map(|z| [z.y_start, z.y_end])
            .filter(|&boundary| boundary > y_raw)
            .min()
            .map(|b| Abs::raw(b as f64))
    }
}
