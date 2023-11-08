// Test lines.

---
// Some simple test lines
#line(length: 60pt, stroke: red)
#v(3pt)
#line(length: 60pt, stroke: 2pt)
#v(3pt)
#line(length: 60pt, stroke: blue + 1.5pt)
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: "dashed"))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 4pt, cap: "round"))

---
// Set rules with stroke
#set line(stroke: (paint: red, thickness: 1pt, cap: "butt", dash: "dash-dotted"))
#line(length: 60pt)
#v(3pt)
#line(length: 60pt, stroke: blue)
#v(3pt)
#line(length: 60pt, stroke: (dash: none))

---
// Rectangle strokes
#rect(width: 20pt, height: 20pt, stroke: red)
#v(3pt)
#rect(width: 20pt, height: 20pt, stroke: (rest: red, top: (paint: blue, dash: "dashed")))
#v(3pt)
#rect(width: 20pt, height: 20pt, stroke: (thickness: 5pt, join: "round"))

---
// Dashing
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: ("dot", 1pt)))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: ("dot", 1pt, 4pt, 2pt)))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: (array: ("dot", 1pt, 4pt, 2pt), phase: 5pt)))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: ()))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: (1pt, 3pt, 9pt)))

---
// Line joins
#stack(
  dir: ltr,
  spacing: 1em,
  polygon(stroke: (thickness: 4pt, paint: blue, join: "round"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  polygon(stroke: (thickness: 4pt, paint: blue, join: "bevel"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  polygon(stroke: (thickness: 4pt, paint: blue, join: "miter"),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
  polygon(stroke: (thickness: 4pt, paint: blue, join: "miter", miter-limit: 20.0),
    (0pt, 20pt), (15pt, 0pt), (0pt, 40pt), (15pt, 45pt)),
)
---
// Error: 29-56 unexpected key "thicknes", valid keys are "paint", "thickness", "cap", "join", "dash", and "miter-limit"
#line(length: 60pt, stroke: (paint: red, thicknes: 1pt))

---
// Error: 29-55 expected "solid", "dotted", "densely-dotted", "loosely-dotted", "dashed", "densely-dashed", "loosely-dashed", "dash-dotted", "densely-dash-dotted", "loosely-dash-dotted", array, dictionary, none, or auto
#line(length: 60pt, stroke: (paint: red, dash: "dash"))

---
// 0pt strokes must function exactly like 'none' strokes and not draw anything
#rect(width: 10pt, height: 10pt, stroke: none)
#rect(width: 10pt, height: 10pt, stroke: 0pt)
#rect(width: 10pt, height: 10pt, stroke: none, fill: blue)
#rect(width: 10pt, height: 10pt, stroke: 0pt + red, fill: blue)

#line(length: 30pt, stroke: 0pt)
#line(length: 30pt, stroke: (paint: red, thickness: 0pt, dash: ("dot", 1pt)))

#table(columns: 2, stroke: none)[A][B]
#table(columns: 2, stroke: 0pt)[A][B]

#path(
  fill: red,
  stroke: none,
  closed: true,
  ((0%, 0%), (4%, -4%)),
  ((50%, 50%), (4%, -4%)),
  ((0%, 50%), (4%, 4%)),
  ((50%, 0%), (4%, 4%)),
)

#path(
  fill: red,
  stroke: 0pt,
  closed: true,
  ((0%, 0%), (4%, -4%)),
  ((50%, 50%), (4%, -4%)),
  ((0%, 50%), (4%, 4%)),
  ((50%, 0%), (4%, 4%)),
)

---
// Converting to stroke
#assert.eq(stroke(red).paint, red)
#assert.eq(stroke(red).thickness, auto)
#assert.eq(stroke(2pt).paint, auto)
#assert.eq(stroke((cap: "round", paint: blue)).cap, "round")
#assert.eq(stroke((cap: auto, paint: blue)).cap, auto)
#assert.eq(stroke((cap: auto, paint: blue)).thickness, auto)

// Error: 9-21 unexpected key "foo", valid keys are "paint", "thickness", "cap", "join", "dash", and "miter-limit"
#stroke((foo: "bar"))

// Constructing with named arguments
#assert.eq(stroke(paint: blue, thickness: 8pt), 8pt + blue)
#assert.eq(stroke(thickness: 2pt), stroke(2pt))
#assert.eq(stroke(cap: "round").thickness, auto)
#assert.eq(stroke(cap: "round", thickness: auto).thickness, auto)
