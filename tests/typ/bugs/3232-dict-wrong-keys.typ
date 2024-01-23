// Issue #3232: Confusing "expected relative length or dictionary, found dictionary"
// https://github.com/typst/typst/issues/3232
// Ref: false

---
// Error: 16-58 unexpected keys "unexpected" and "unexpected-too", valid keys are "left", "top", "right", "bottom", "x", "y", and "rest"
#block(outset: (unexpected: 0.5em, unexpected-too: 0.2em), [Hi])

---
// Error: 14-56 unexpected keys "unexpected" and "unexpected-too", valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#box(radius: (unexpected: 0.5em, unexpected-too: 0.5em), [Hi])

---
// Error: 16-49 unexpected key "unexpected", valid keys are "left", "top", "right", "bottom", "x", "y", and "rest"
#block(outset: (unexpected: 0.2em, right: 0.5em), [Hi]) // The 1st key is unexpected

---
// Error: 14-50 unexpected key "unexpected", valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#box(radius: (top-left: 0.5em, unexpected: 0.5em), [Hi]) // The 2nd key is unexpected

---
// Error: 16-19 unexpectedly empty, valid keys are "left", "top", "right", "bottom", "x", "y", and "rest"
#block(outset: (:), [Hi])

---
// Error: 14-17 unexpectedly empty, valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#box(radius: (:), [Hi])
