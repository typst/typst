// Test lines.

---
// Some simple test lines

#line(length: 60pt, stroke: red)
#v(3pt)
#line(length: 60pt, stroke: 2pt)
#v(3pt)
#line(length: 60pt, stroke: blue + 1.5pt)
#v(3pt)
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: "dashed"))
#v(3pt)
#line(length: 60pt, stroke: (color: red, thickness: 4pt, line_cap: "round"))

---
// Set rules with stroke

#set line(stroke: (color: red, thickness: 1pt, line_cap: "butt", dash: "dashdotted"))

#line(length: 60pt)
#v(3pt)
#line(length: 60pt, stroke: blue)
#v(3pt)
#line(length: 60pt, stroke: (dash: none))

---
// Rectangle strokes

#rect(width: 20pt, height: 20pt, stroke: red)
#v(3pt)
#rect(width: 20pt, height: 20pt, stroke: (rest: red, top: (color: blue, dash: "dashed")))
#v(3pt)
#rect(width: 20pt, height: 20pt, stroke: (thickness: 5pt, line_join: "round"))

---
// Dashing
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: ("dot", 1pt)))
#v(3pt)
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: ("dot", 1pt, 4pt, 2pt)))
#v(3pt)
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: (array: ("dot", 1pt, 4pt, 2pt), phase: 5pt)))

---

// Error: 29-56 unexpected key "thicknes", valid keys are "color", "thickness", "line_cap", "line_join", and "dash"
#line(length: 60pt, stroke: (color: red, thicknes: 1pt))

---

// Error: 29-55 expected "solid", "dotted", "densely dotted", "loosely dotted", "dashed", "densely dashed", "loosely dashed", "dashdotted", "densely dashdotted", "loosely dashdotted", array, dictionary, dash pattern, or none
#line(length: 60pt, stroke: (color: red, dash: "dash"))
