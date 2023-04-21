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
#line(length: 60pt, stroke: (color: red, thickness: 4pt, cap: "round"))

---
// Set rules with stroke

#set line(stroke: (color: red, thickness: 1pt, cap: "butt", dash: "dashdotted"))

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
#rect(width: 20pt, height: 20pt, stroke: (thickness: 5pt, join: "round"))

---
// Dashing
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: ("dot", 1pt)))
#v(3pt)
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: ("dot", 1pt, 4pt, 2pt)))
#v(3pt)
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: (array: ("dot", 1pt, 4pt, 2pt), phase: 5pt)))
#v(3pt)
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: ()))
#v(3pt)
#line(length: 60pt, stroke: (color: red, thickness: 1pt, dash: (1pt, 3pt, 9pt)))

---
// Line joins
#stack(dir: ltr, 
  polygon(stroke: (thickness: 4pt, color: blue, join: "round"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  h(1em),
  polygon(stroke: (thickness: 4pt, color: blue, join: "bevel"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  h(1em),
  polygon(stroke: (thickness: 4pt, color: blue, join: "miter"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  h(1em),
  polygon(stroke: (thickness: 4pt, color: blue, join: "miter", miter-limit: 20.0),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
)
---

// Error: 29-56 unexpected key "thicknes", valid keys are "color", "thickness", "cap", "join", "dash", and "miter-limit"
#line(length: 60pt, stroke: (color: red, thicknes: 1pt))

---

// Error: 29-55 expected "solid", "dotted", "densely-dotted", "loosely-dotted", "dashed", "densely-dashed", "loosely-dashed", "dashdotted", "densely-dashdotted", "loosely-dashdotted", array, dictionary, dash pattern, or none
#line(length: 60pt, stroke: (color: red, dash: "dash"))

---
// 0pt strokes must function exactly like 'none' strokes and not draw anything
#rect(width: 10pt, height: 10pt, stroke: none)
#rect(width: 10pt, height: 10pt, stroke: 0pt)

#rect(width: 10pt, height: 10pt, stroke: none, fill: blue)
#rect(width: 10pt, height: 10pt, stroke: 0pt + red, fill: blue)

#line(length: 30pt, stroke: 0pt)
#line(length: 30pt, stroke: (color: red, thickness: 0pt, dash: ("dot", 1pt)))

#table(columns: 2, stroke: none)[A][B]
#table(columns: 2, stroke: 0pt)[A][B]
