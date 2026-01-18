use std::fmt::{self, Debug, Formatter};

use crate::layout::{Abs, Axes, FixedAlignment, Frame, Size};

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
/// in sorted lookups. The vertical range is half-open: `[y_start, y_end)`,
/// meaning `y_start` is inclusive and `y_end` is exclusive.
///
/// # Coordinate System
///
/// All coordinates are in paragraph-relative space (y=0 at paragraph top).
/// Use [`ExclusionZone::new`] to construct from `Abs` values, which handles
/// the conversion to raw units consistently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExclusionZone {
    /// Y-offset from paragraph top where exclusion starts (inclusive, in raw units).
    pub y_start: i64,
    /// Y-offset from paragraph top where exclusion ends (exclusive, in raw units).
    pub y_end: i64,
    /// Width excluded from left side (in raw units).
    pub left: i64,
    /// Width excluded from right side (in raw units).
    pub right: i64,
}

impl ExclusionZone {
    /// Create an exclusion zone from `Abs` coordinates.
    ///
    /// This is the preferred constructor as it ensures consistent rounding
    /// from floating-point `Abs` values to integer raw units.
    ///
    /// # Arguments
    /// * `y_start` - Y-offset where exclusion starts (inclusive)
    /// * `y_end` - Y-offset where exclusion ends (exclusive)
    /// * `left` - Width to exclude from left side
    /// * `right` - Width to exclude from right side
    pub fn new(y_start: Abs, y_end: Abs, left: Abs, right: Abs) -> Self {
        Self {
            y_start: abs_to_raw(y_start),
            y_end: abs_to_raw(y_end),
            left: abs_to_raw(left),
            right: abs_to_raw(right),
        }
    }
}

/// Convert an `Abs` value to raw i64 units with consistent rounding.
///
/// This centralizes the rounding strategy (round-to-nearest) to ensure
/// all Abs-to-raw conversions in the exclusion system behave identically.
#[inline]
fn abs_to_raw(value: Abs) -> i64 {
    value.to_raw().round() as i64
}

/// Convert raw i64 units back to `Abs`.
///
/// This is the inverse of [`abs_to_raw`]. Note that due to rounding in
/// `abs_to_raw`, the round-trip `abs_to_raw(raw_to_abs(x))` may not equal `x`
/// for values that were not originally derived from `Abs`.
#[inline]
fn raw_to_abs(value: i64) -> Abs {
    Abs::raw(value as f64)
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

impl WrapFloat {
    /// Create a wrap-float from a placed element's frame and positioning.
    ///
    /// Computes which margins to exclude based on horizontal alignment:
    /// - Start/Left aligned: exclude from left (text wraps on right)
    /// - End/Right aligned: exclude from right (text wraps on left)
    /// - Center aligned: exclude from both sides (experimental)
    pub fn from_placed(
        frame: &Frame,
        y: Abs,
        align_x: FixedAlignment,
        clearance: Abs,
    ) -> Self {
        // Clamp clearance to zero to prevent negative margins
        let width = frame.width() + clearance.max(Abs::zero());
        let (left_margin, right_margin) = match align_x {
            FixedAlignment::Start => (width, Abs::zero()),
            FixedAlignment::End => (Abs::zero(), width),
            FixedAlignment::Center => {
                // Center-aligned wrap-floats exclude from both sides
                let half = width / 2.0;
                (half, half)
            }
        };
        Self {
            y,
            height: frame.height(),
            left_margin,
            right_margin,
        }
    }
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

            // Convert to paragraph-relative coordinates, clamped to paragraph bounds.
            // We work in raw f64 space here to avoid accumulating rounding errors
            // from multiple Abs operations, then round once at the end.
            let rel_start = Abs::raw((wf_top - par_top).max(0.0));
            let rel_end = Abs::raw((wf_bottom - par_top).min(par_bottom - par_top));

            zones.push(ExclusionZone::new(
                rel_start,
                rel_end,
                wf.left_margin,
                wf.right_margin,
            ));
        }

        // Sort by y_start for efficient lookup
        zones.sort_by_key(|z| z.y_start);

        Self { zones }
    }

    /// Get available width at a given y-offset within the paragraph.
    ///
    /// If multiple exclusions overlap at this y, takes the maximum from each side.
    pub fn available_width(&self, base_width: Abs, y: Abs) -> Abs {
        let y_raw = abs_to_raw(y);
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

        let total_excluded = raw_to_abs(left_excluded) + raw_to_abs(right_excluded);
        (base_width - total_excluded).max(Abs::zero())
    }

    /// Get left offset at a given y-offset (for text positioning).
    pub fn left_offset(&self, y: Abs) -> Abs {
        let y_raw = abs_to_raw(y);
        let mut left = 0i64;

        for zone in &self.zones {
            if y_raw < zone.y_start {
                break;
            }
            if y_raw < zone.y_end {
                left = left.max(zone.left);
            }
        }

        raw_to_abs(left)
    }

    /// Check if any exclusion is active at this y-offset.
    pub fn has_exclusion_at(&self, y: Abs) -> bool {
        let y_raw = abs_to_raw(y);
        self.zones.iter().any(|z| y_raw >= z.y_start && y_raw < z.y_end)
    }

    /// Get the next y-position where exclusions change (for line breaking).
    ///
    /// Returns the next y where an exclusion starts or ends, enabling the
    /// line-breaking algorithm to skip to known boundary points.
    pub fn next_boundary(&self, y: Abs) -> Option<Abs> {
        let y_raw = abs_to_raw(y);
        self.zones
            .iter()
            .flat_map(|z| [z.y_start, z.y_end])
            .filter(|&boundary| boundary > y_raw)
            .min()
            .map(raw_to_abs)
    }
}

#[cfg(test)]
mod exclusion_tests {
    use super::*;

    // Helper to create Abs from pt for cleaner test code
    fn pt(value: f64) -> Abs {
        Abs::pt(value)
    }

    // ========================================
    // ParExclusions::available_width tests
    // ========================================

    #[test]
    fn test_available_width_before_exclusion() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(10.0), pt(50.0), pt(30.0), Abs::zero())],
        };

        let base = pt(200.0);
        // Query at y=5, before exclusion starts at y=10
        let width = excl.available_width(base, pt(5.0));
        assert_eq!(width, pt(200.0));
    }

    #[test]
    fn test_available_width_during_exclusion() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(10.0), pt(50.0), pt(30.0), Abs::zero())],
        };

        let base = pt(200.0);
        // Query at y=20, during exclusion (10-50)
        let width = excl.available_width(base, pt(20.0));
        assert_eq!(width, pt(170.0)); // 200 - 30 = 170
    }

    #[test]
    fn test_available_width_after_exclusion() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(10.0), pt(50.0), pt(30.0), Abs::zero())],
        };

        let base = pt(200.0);
        // Query at y=60, after exclusion ends at y=50
        let width = excl.available_width(base, pt(60.0));
        assert_eq!(width, pt(200.0));
    }

    #[test]
    fn test_available_width_both_sides() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(0.0), pt(100.0), pt(40.0), pt(60.0))],
        };

        let base = pt(300.0);
        let width = excl.available_width(base, pt(50.0));
        assert_eq!(width, pt(200.0)); // 300 - 40 - 60 = 200
    }

    #[test]
    fn test_available_width_overlapping_zones() {
        // Two overlapping zones - should take max from each side
        let excl = ParExclusions {
            zones: vec![
                ExclusionZone::new(pt(10.0), pt(50.0), pt(30.0), Abs::zero()),
                ExclusionZone::new(pt(20.0), pt(40.0), pt(50.0), pt(20.0)), // Larger left
            ],
        };

        let base = pt(200.0);
        // At y=25, both zones active: max(30, 50)=50 left, max(0, 20)=20 right
        let width = excl.available_width(base, pt(25.0));
        assert_eq!(width, pt(130.0)); // 200 - 50 - 20 = 130
    }

    #[test]
    fn test_available_width_empty_exclusions() {
        let excl = ParExclusions::default();
        let base = pt(200.0);
        // Empty exclusions should return full width
        assert_eq!(excl.available_width(base, pt(0.0)), pt(200.0));
        assert_eq!(excl.available_width(base, pt(100.0)), pt(200.0));
    }

    // ========================================
    // ParExclusions::from_wrap_floats tests
    // ========================================

    #[test]
    fn test_from_wrap_floats_basic_overlap() {
        let floats = vec![WrapFloat {
            y: pt(20.0),
            height: pt(40.0), // Extends from y=20 to y=60
            left_margin: Abs::zero(),
            right_margin: pt(50.0),
        }];

        // Paragraph at y=30, height=100 (overlaps float 20-60)
        let excl = ParExclusions::from_wrap_floats(pt(30.0), pt(100.0), &floats);

        assert_eq!(excl.zones.len(), 1);
        let zone = &excl.zones[0];
        // Float runs from 20-60, paragraph starts at 30
        // Exclusion starts at max(20-30, 0) = 0 (clamped)
        // Exclusion ends at min(60-30, 100) = 30
        assert_eq!(zone.y_start, 0);
        assert_eq!(zone.y_end, pt(30.0).to_raw().round() as i64);
        assert_eq!(zone.right, pt(50.0).to_raw().round() as i64);
    }

    #[test]
    fn test_from_wrap_floats_no_overlap() {
        let floats = vec![WrapFloat {
            y: pt(100.0),
            height: pt(40.0), // Extends from y=100 to y=140
            left_margin: pt(50.0),
            right_margin: Abs::zero(),
        }];

        // Paragraph at y=0, height=50 (doesn't overlap float at 100-140)
        let excl = ParExclusions::from_wrap_floats(pt(0.0), pt(50.0), &floats);

        assert!(excl.is_empty());
    }

    #[test]
    fn test_from_wrap_floats_float_before_paragraph() {
        let floats = vec![WrapFloat {
            y: pt(0.0),
            height: pt(30.0), // Extends from y=0 to y=30
            left_margin: pt(40.0),
            right_margin: Abs::zero(),
        }];

        // Paragraph at y=50 (after float ends)
        let excl = ParExclusions::from_wrap_floats(pt(50.0), pt(100.0), &floats);

        assert!(excl.is_empty());
    }

    #[test]
    fn test_from_wrap_floats_float_fully_inside_paragraph() {
        let floats = vec![WrapFloat {
            y: pt(50.0),
            height: pt(30.0), // Extends from y=50 to y=80
            left_margin: pt(60.0),
            right_margin: Abs::zero(),
        }];

        // Paragraph at y=0, height=200 (fully contains float)
        let excl = ParExclusions::from_wrap_floats(pt(0.0), pt(200.0), &floats);

        assert_eq!(excl.zones.len(), 1);
        let zone = &excl.zones[0];
        // Float is at 50-80, paragraph starts at 0
        // Exclusion from 50 to 80 in paragraph coords
        assert_eq!(zone.y_start, pt(50.0).to_raw().round() as i64);
        assert_eq!(zone.y_end, pt(80.0).to_raw().round() as i64);
    }

    #[test]
    fn test_from_wrap_floats_multiple_floats() {
        let floats = vec![
            WrapFloat {
                y: pt(10.0),
                height: pt(20.0), // 10-30
                left_margin: pt(30.0),
                right_margin: Abs::zero(),
            },
            WrapFloat {
                y: pt(50.0),
                height: pt(20.0), // 50-70
                left_margin: Abs::zero(),
                right_margin: pt(40.0),
            },
        ];

        // Paragraph at y=0, height=100 (overlaps both floats)
        let excl = ParExclusions::from_wrap_floats(pt(0.0), pt(100.0), &floats);

        assert_eq!(excl.zones.len(), 2);
        // Zones should be sorted by y_start
        assert!(excl.zones[0].y_start <= excl.zones[1].y_start);
    }

    #[test]
    fn test_from_wrap_floats_zones_sorted() {
        // Create floats out of order
        let floats = vec![
            WrapFloat {
                y: pt(80.0),
                height: pt(20.0),
                left_margin: pt(30.0),
                right_margin: Abs::zero(),
            },
            WrapFloat {
                y: pt(20.0),
                height: pt(20.0),
                left_margin: pt(30.0),
                right_margin: Abs::zero(),
            },
            WrapFloat {
                y: pt(50.0),
                height: pt(20.0),
                left_margin: pt(30.0),
                right_margin: Abs::zero(),
            },
        ];

        let excl = ParExclusions::from_wrap_floats(pt(0.0), pt(200.0), &floats);

        assert_eq!(excl.zones.len(), 3);
        // Verify sorted order
        assert!(excl.zones[0].y_start <= excl.zones[1].y_start);
        assert!(excl.zones[1].y_start <= excl.zones[2].y_start);
    }

    // ========================================
    // ParExclusions::left_offset tests
    // ========================================

    #[test]
    fn test_left_offset_no_exclusion() {
        let excl = ParExclusions::default();
        assert_eq!(excl.left_offset(pt(50.0)), Abs::zero());
    }

    #[test]
    fn test_left_offset_during_exclusion() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(10.0), pt(50.0), pt(30.0), pt(20.0))],
        };

        // During exclusion
        assert_eq!(excl.left_offset(pt(25.0)), pt(30.0));
        // Before exclusion
        assert_eq!(excl.left_offset(pt(5.0)), Abs::zero());
        // After exclusion
        assert_eq!(excl.left_offset(pt(60.0)), Abs::zero());
    }

    // ========================================
    // ParExclusions::has_exclusion_at tests
    // ========================================

    #[test]
    fn test_has_exclusion_at() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(10.0), pt(50.0), pt(30.0), Abs::zero())],
        };

        assert!(!excl.has_exclusion_at(pt(5.0))); // Before
        assert!(excl.has_exclusion_at(pt(10.0))); // At start (inclusive)
        assert!(excl.has_exclusion_at(pt(30.0))); // During
        assert!(!excl.has_exclusion_at(pt(50.0))); // At end (exclusive)
        assert!(!excl.has_exclusion_at(pt(60.0))); // After
    }

    // ========================================
    // ParExclusions::next_boundary tests
    // ========================================

    #[test]
    fn test_next_boundary_before_zone() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(20.0), pt(50.0), pt(30.0), Abs::zero())],
        };

        // Query before zone: should return y_start
        let boundary = excl.next_boundary(pt(10.0));
        assert_eq!(boundary, Some(pt(20.0)));
    }

    #[test]
    fn test_next_boundary_inside_zone() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(20.0), pt(50.0), pt(30.0), Abs::zero())],
        };

        // Query inside zone: should return y_end
        let boundary = excl.next_boundary(pt(30.0));
        assert_eq!(boundary, Some(pt(50.0)));
    }

    #[test]
    fn test_next_boundary_after_all_zones() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(pt(20.0), pt(50.0), pt(30.0), Abs::zero())],
        };

        // Query after zone: no more boundaries
        let boundary = excl.next_boundary(pt(60.0));
        assert_eq!(boundary, None);
    }

    #[test]
    fn test_next_boundary_multiple_zones() {
        let excl = ParExclusions {
            zones: vec![
                ExclusionZone::new(pt(10.0), pt(30.0), pt(20.0), Abs::zero()),
                ExclusionZone::new(pt(50.0), pt(70.0), pt(20.0), Abs::zero()),
            ],
        };

        // Before first zone: get start of first
        assert_eq!(excl.next_boundary(pt(5.0)), Some(pt(10.0)));
        // Inside first zone: get end of first
        assert_eq!(excl.next_boundary(pt(15.0)), Some(pt(30.0)));
        // Between zones: get start of second
        assert_eq!(excl.next_boundary(pt(40.0)), Some(pt(50.0)));
        // Inside second zone: get end of second
        assert_eq!(excl.next_boundary(pt(60.0)), Some(pt(70.0)));
        // After all zones: none
        assert_eq!(excl.next_boundary(pt(80.0)), None);
    }

    #[test]
    fn test_next_boundary_empty_exclusions() {
        let excl = ParExclusions::default();
        assert_eq!(excl.next_boundary(pt(0.0)), None);
    }

    // ========================================
    // WrapFloat::from_placed tests
    // ========================================

    #[test]
    fn test_wrap_float_from_placed_start_aligned() {
        let mut frame = Frame::soft(Size::new(pt(80.0), pt(100.0)));
        frame.set_size(Size::new(pt(80.0), pt(100.0)));

        let wf = WrapFloat::from_placed(&frame, pt(50.0), FixedAlignment::Start, pt(10.0));

        assert_eq!(wf.y, pt(50.0));
        assert_eq!(wf.height, pt(100.0));
        assert_eq!(wf.left_margin, pt(90.0)); // 80 + 10 clearance
        assert_eq!(wf.right_margin, Abs::zero());
    }

    #[test]
    fn test_wrap_float_from_placed_end_aligned() {
        let mut frame = Frame::soft(Size::new(pt(80.0), pt(100.0)));
        frame.set_size(Size::new(pt(80.0), pt(100.0)));

        let wf = WrapFloat::from_placed(&frame, pt(50.0), FixedAlignment::End, pt(10.0));

        assert_eq!(wf.y, pt(50.0));
        assert_eq!(wf.height, pt(100.0));
        assert_eq!(wf.left_margin, Abs::zero());
        assert_eq!(wf.right_margin, pt(90.0)); // 80 + 10 clearance
    }

    #[test]
    fn test_wrap_float_from_placed_center_aligned() {
        let mut frame = Frame::soft(Size::new(pt(80.0), pt(100.0)));
        frame.set_size(Size::new(pt(80.0), pt(100.0)));

        let wf = WrapFloat::from_placed(&frame, pt(50.0), FixedAlignment::Center, pt(10.0));

        assert_eq!(wf.y, pt(50.0));
        assert_eq!(wf.height, pt(100.0));
        // Center: (80 + 10) / 2 = 45 each side
        assert_eq!(wf.left_margin, pt(45.0));
        assert_eq!(wf.right_margin, pt(45.0));
    }

    // ========================================
    // Edge cases
    // ========================================

    #[test]
    fn test_is_empty() {
        assert!(ParExclusions::default().is_empty());
        assert!(ParExclusions { zones: vec![] }.is_empty());

        let non_empty = ParExclusions {
            zones: vec![ExclusionZone::new(Abs::zero(), pt(100.0), pt(50.0), Abs::zero())],
        };
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_available_width_clamped_to_zero() {
        // Exclusions larger than base width
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(Abs::zero(), pt(100.0), pt(150.0), pt(100.0))],
        };

        let base = pt(200.0);
        // 150 + 100 = 250 > 200, should clamp to zero
        let width = excl.available_width(base, pt(50.0));
        assert_eq!(width, Abs::zero());
    }

    #[test]
    fn test_exclusion_zone_only_left() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(Abs::zero(), pt(100.0), pt(50.0), Abs::zero())],
        };

        let base = pt(200.0);
        assert_eq!(excl.available_width(base, pt(50.0)), pt(150.0));
        assert_eq!(excl.left_offset(pt(50.0)), pt(50.0));
    }

    #[test]
    fn test_exclusion_zone_only_right() {
        let excl = ParExclusions {
            zones: vec![ExclusionZone::new(Abs::zero(), pt(100.0), Abs::zero(), pt(50.0))],
        };

        let base = pt(200.0);
        assert_eq!(excl.available_width(base, pt(50.0)), pt(150.0));
        assert_eq!(excl.left_offset(pt(50.0)), Abs::zero()); // No left offset
    }

    // ========================================
    // ExclusionZone::new tests
    // ========================================

    #[test]
    fn test_exclusion_zone_new_converts_correctly() {
        let zone = ExclusionZone::new(pt(10.0), pt(50.0), pt(30.0), pt(20.0));

        // Verify that the constructor produces the same result as manual conversion
        assert_eq!(zone.y_start, abs_to_raw(pt(10.0)));
        assert_eq!(zone.y_end, abs_to_raw(pt(50.0)));
        assert_eq!(zone.left, abs_to_raw(pt(30.0)));
        assert_eq!(zone.right, abs_to_raw(pt(20.0)));
    }

    #[test]
    fn test_abs_to_raw_round_trip() {
        // Verify that round-trip conversion is stable for values that came from Abs
        let original = pt(123.456);
        let raw = abs_to_raw(original);
        let back = raw_to_abs(raw);
        let raw_again = abs_to_raw(back);

        // After one round-trip, further conversions should be stable
        assert_eq!(raw, raw_again);
    }
}
