# Phase 2: Exclusion Data Structures

**Goal:** Implement the data structures for tracking wrap-float geometry and computing text exclusion zones.

## Coordinate System Specification

All coordinates in this system are relative to the **inner flow origin**:

```
┌─────────────────────────────────────────┐
│  Page                                   │
│  ┌───────────────────────────────────┐  │
│  │  Top Insertions (floats)          │  │  ← page_insertions.top_size
│  ├───────────────────────────────────┤  │
│  │  Inner Flow Origin (y=0)    ──────┼──┼── This is the coordinate origin
│  │  ┌─────────────────────────────┐  │  │
│  │  │  Content Region             │  │  │
│  │  │                             │  │  │
│  │  │    Paragraph at y=50pt      │  │  │  ← ParExclusions use this y
│  │  │    ┌─────────┐              │  │  │
│  │  │    │ WrapFloat│  Text wraps │  │  │  ← WrapFloat.y is relative to
│  │  │    │ at y=30pt│  around it  │  │  │     inner flow origin
│  │  │    └─────────┘              │  │  │
│  │  │                             │  │  │
│  │  └─────────────────────────────┘  │  │
│  ├───────────────────────────────────┤  │
│  │  Bottom Insertions (floats, fn)   │  │  ← page_insertions.bottom_size
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

**Key invariants:**

1. `WrapFloat.y` is in inner-flow coordinates (y=0 at top of content region)
2. `ParExclusions` zones are in paragraph-relative coordinates (y=0 at paragraph top)
3. When computing exclusions for a paragraph at `par_y`, transform:
   `exclusion_zone.y_start = wrap_float.y - par_y`
4. Footnotes/bottom insertions reduce available height but don't change y=0

## ParExclusions Type

**File: `crates/typst-library/src/layout/regions.rs`**

*Find insertion point:* Search for `pub struct Regions` or end of file.
These are new types to add.

```rust
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExclusionZone {
    /// Y-offset from paragraph top where exclusion starts (in raw Abs units).
    pub y_start: i64,
    /// Y-offset from paragraph top where exclusion ends (in raw Abs units).
    pub y_end: i64,
    /// Width excluded from left side (in raw Abs units).
    pub left: i64,
    /// Width excluded from right side (in raw Abs units).
    pub right: i64,
}
```

## ParExclusions Methods

```rust
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

            // Convert to paragraph-relative coordinates
            let rel_start = (wf_top - par_top).max(0);
            let rel_end = (wf_bottom - par_top).min(par_bottom - par_top);

            zones.push(ExclusionZone {
                y_start: rel_start,
                y_end: rel_end,
                left: wf.left_margin.to_raw(),
                right: wf.right_margin.to_raw(),
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
        let y_raw = y.to_raw();
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

        let total_excluded = Abs::from_raw(left_excluded) + Abs::from_raw(right_excluded);
        (base_width - total_excluded).max(Abs::zero())
    }

    /// Get left offset at a given y-offset (for text positioning).
    pub fn left_offset(&self, y: Abs) -> Abs {
        let y_raw = y.to_raw();
        let mut left = 0i64;

        for zone in &self.zones {
            if y_raw < zone.y_start {
                break;
            }
            if y_raw < zone.y_end {
                left = left.max(zone.left);
            }
        }

        Abs::from_raw(left)
    }

    /// Check if any exclusion is active at this y-offset.
    pub fn has_exclusion_at(&self, y: Abs) -> bool {
        let y_raw = y.to_raw();
        self.zones.iter().any(|z| y_raw >= z.y_start && y_raw < z.y_end)
    }

    /// Get the next y-position where exclusions change (for line breaking).
    pub fn next_boundary(&self, y: Abs) -> Option<Abs> {
        let y_raw = y.to_raw();
        self.zones
            .iter()
            .flat_map(|z| [z.y_start, z.y_end])
            .filter(|&boundary| boundary > y_raw)
            .min()
            .map(Abs::from_raw)
    }
}
```

## WrapFloat Type

```rust
/// A positioned wrap-float in region coordinates.
#[derive(Debug, Clone)]
pub struct WrapFloat {
    /// Top y-coordinate in region (inner-flow) coordinates.
    pub y: Abs,
    /// Height of the float.
    pub height: Abs,
    /// Width excluded from left (float width + clearance, or 0 if right-aligned).
    pub left_margin: Abs,
    /// Width excluded from right (float width + clearance, or 0 if left-aligned).
    pub right_margin: Abs,
}

impl WrapFloat {
    /// Create a wrap-float from a placed child and its computed position.
    pub fn from_placed(
        frame: &Frame,
        y: Abs,
        align_x: FixedAlignment,
        clearance: Abs,
    ) -> Self {
        let width = frame.width() + clearance;
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
```

## Wrap Parameter on PlaceElem

**File: `crates/typst-library/src/layout/place.rs`**

*Find with:* `grep -n "pub struct PlaceElem" crates/typst-library/src/layout/place.rs`
*Or find existing params:* `grep -n "pub float:" crates/typst-library/src/layout/place.rs`

Add to `PlaceElem` struct (near the `float` parameter):

```rust
/// Whether text should wrap around this floating element.
///
/// When enabled with `float: true`, paragraphs will have shortened
/// lines adjacent to this element. Only effective when horizontal
/// alignment is `left` or `right` (center-aligned wrapping is experimental).
///
/// ```example
/// #set page(height: 200pt)
/// #place(
///   top + right,
///   float: true,
///   wrap: true,
///   clearance: 10pt,
///   rect(width: 60pt, height: 80pt, fill: aqua),
/// )
/// #lorem(50)
/// ```
#[default(false)]
pub wrap: bool,
```

## Unit Tests

**File: `crates/typst-library/src/layout/regions_test.rs`** (or inline)

```rust
#[test]
fn test_exclusion_available_width() {
    let excl = ParExclusions {
        zones: vec![
            ExclusionZone {
                y_start: Abs::pt(10.0).to_raw(),
                y_end: Abs::pt(50.0).to_raw(),
                left: Abs::pt(30.0).to_raw(),
                right: 0,
            },
        ],
    };

    let base = Abs::pt(200.0);

    // Before exclusion
    assert_eq!(excl.available_width(base, Abs::pt(5.0)), Abs::pt(200.0));

    // During exclusion
    assert_eq!(excl.available_width(base, Abs::pt(20.0)), Abs::pt(170.0));

    // After exclusion
    assert_eq!(excl.available_width(base, Abs::pt(60.0)), Abs::pt(200.0));
}

#[test]
fn test_exclusion_from_wrap_floats() {
    let floats = vec![
        WrapFloat {
            y: Abs::pt(20.0),
            height: Abs::pt(40.0),
            left_margin: Abs::zero(),
            right_margin: Abs::pt(50.0),
        },
    ];

    // Paragraph starting at y=30 (overlaps float)
    let excl = ParExclusions::from_wrap_floats(
        Abs::pt(30.0),
        Abs::pt(100.0),
        &floats,
    );

    assert_eq!(excl.zones.len(), 1);
    // Float runs from 20-60, paragraph starts at 30
    // So exclusion runs from 0 (30-30) to 30 (60-30)
    assert_eq!(excl.zones[0].y_start, 0);
    assert_eq!(excl.zones[0].y_end, Abs::pt(30.0).to_raw());
}

#[test]
fn test_exclusion_no_overlap() {
    let floats = vec![
        WrapFloat {
            y: Abs::pt(100.0),
            height: Abs::pt(40.0),
            left_margin: Abs::pt(50.0),
            right_margin: Abs::zero(),
        },
    ];

    // Paragraph at y=0, height=50 (doesn't overlap float at y=100)
    let excl = ParExclusions::from_wrap_floats(
        Abs::pt(0.0),
        Abs::pt(50.0),
        &floats,
    );

    assert!(excl.is_empty());
}
```

## Exit Criteria

- [ ] `ParExclusions::available_width()` correctly computes width at any y
- [ ] `ParExclusions::from_wrap_floats()` correctly transforms coordinates
- [ ] Coordinate system is documented and consistent
- [ ] Unit tests pass for all edge cases
- [ ] `wrap` parameter parses correctly on `PlaceElem`

## Dependencies

- [Phase 1: ParChild Structure](WRAP_PHASE_1.md) must be complete

## Next Phase

[Phase 3: Distribution Changes](WRAP_PHASE_3.md)
