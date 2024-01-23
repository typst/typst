// Issue #3232: Confusing error message, "expected a or b, found b"
// https://github.com/typst/typst/issues/3232
// Ref: false

---
// Error: 16-45 unexpected keys "before" and "other", valid keys are "left", "top", "right", "bottom", "x", "y", and "rest"
#block(outset: (before: 0.5em, other: 0.2em), [Hi])

---
// The 1st key is expected, but the 2nd is not.
// Error: 16-43 unexpected key "other", valid keys are "left", "top", "right", "bottom", "x", "y", and "rest"
#block(outset: (left: 0.5em, other: 0.2em), [Hi])

---
#block(outset: (left: 0.5em, rest: 0.2em), [Hi])

---
// Error: 14-52 unexpected keys "top_left" and "bottom_right", valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#box(radius: (top_left: 0.5em, bottom_right: 0.5em), [Hi])

---
// The 2nd key is expected, but the 1st is not.
// Error: 14-46 unexpected key "center", valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#box(radius: (center: 0.5em, top-left: 0.5em), [Hi])

---
#box(radius: (top-left: 0.5em, bottom-right: 0.5em), [Hi])
