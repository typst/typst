// Test lines.

--- stroke-constructor ---
// Converting to stroke
#test(stroke(red).paint, red)
#test(stroke(red).thickness, auto)
#test(stroke(2pt).paint, auto)
#test(stroke((cap: "round", paint: blue)).cap, "round")
#test(stroke((cap: auto, paint: blue)).cap, auto)
#test(stroke((cap: auto, paint: blue)).thickness, auto)

// Constructing with named arguments
#test(stroke(paint: blue, thickness: 8pt), 8pt + blue)
#test(stroke(thickness: 2pt), stroke(2pt))
#test(stroke(cap: "round").thickness, auto)
#test(stroke(cap: "round", thickness: auto).thickness, auto)

--- stroke-constructor-unknown-key ---
// Error: 9-21 unexpected key "foo", valid keys are "paint", "thickness", "cap", "join", "dash", and "miter-limit"
#stroke((foo: "bar"))

--- stroke-fields-simple ---
// Test stroke fields for simple strokes.
#test((1em + blue).paint, blue)
#test((1em + blue).thickness, 1em)
#test((1em + blue).cap, auto)
#test((1em + blue).join, auto)
#test((1em + blue).dash, auto)
#test((1em + blue).miter-limit, auto)

--- stroke-fields-complex ---
// Test complex stroke fields.
#let r1 = rect(stroke: (paint: cmyk(1%, 2%, 3%, 4%), thickness: 4em + 2pt, cap: "round", join: "bevel", miter-limit: 5.0, dash: none))
#let r2 = rect(stroke: (paint: cmyk(1%, 2%, 3%, 4%), thickness: 4em + 2pt, cap: "round", join: "bevel", miter-limit: 5.0, dash: (3pt, "dot", 4em)))
#let r3 = rect(stroke: (paint: cmyk(1%, 2%, 3%, 4%), thickness: 4em + 2pt, cap: "round", join: "bevel", dash: (array: (3pt, "dot", 4em), phase: 5em)))
#let s1 = r1.stroke
#let s2 = r2.stroke
#let s3 = r3.stroke
#test(s1.paint, cmyk(1%, 2%, 3%, 4%))
#test(s1.thickness, 4em + 2pt)
#test(s1.cap, "round")
#test(s1.join, "bevel")
#test(s1.miter-limit, 5.0)
#test(s3.miter-limit, auto)
#test(s1.dash, none)
#test(s2.dash, (array: (3pt, "dot", 4em), phase: 0pt))
#test(s3.dash, (array: (3pt, "dot", 4em), phase: 5em))

--- stroke-zero-thickness ---
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

--- stroke-text ---
#set text(size: 20pt)
#set page(width: auto)
#let v = [测试字体Test]

#text(stroke: 0.3pt + red, v)

#text(stroke: 0.7pt + red, v)

#text(stroke: 7pt + red, v)

#text(stroke: (paint: blue, thickness: 1pt, dash: "dashed"), v)

#text(stroke: 1pt + gradient.linear(..color.map.rainbow), v)

--- stroke-folding ---
// Test stroke folding.
#let sq(..args) = box(square(size: 10pt, ..args))

#set square(stroke: none)
#sq()
#set square(stroke: auto)
#sq()
#sq(fill: teal)
#sq(stroke: 2pt)
#sq(stroke: blue)
#sq(fill: teal, stroke: blue)
#sq(fill: teal, stroke: 2pt + blue)

--- stroke-composition ---
// Test stroke composition.
#set square(stroke: 4pt)
#set text(font: "Roboto")
#stack(
  dir: ltr,
  square(
    stroke: (left: red, top: yellow, right: green, bottom: blue),
    radius: 50%, align(center+horizon)[*G*],
    inset: 8pt
  ),
  h(0.5cm),
  square(
    stroke: (left: red, top: yellow + 8pt, right: green, bottom: blue + 2pt),
    radius: 50%, align(center+horizon)[*G*],
    inset: 8pt
  ),
  h(0.5cm),
  square(
    stroke: (left: red, top: yellow, right: green, bottom: blue),
    radius: 100%, align(center+horizon)[*G*],
    inset: 8pt
  ),
)

// Join between different solid strokes
#set square(size: 20pt, stroke: 2pt)
#set square(stroke: (left: green + 4pt, top: black + 2pt, right: blue, bottom: black + 2pt))
#stack(
  dir: ltr,
  square(),
  h(0.2cm),
  square(radius: (top-left: 0pt, rest: 1pt)),
  h(0.2cm),
  square(radius: (top-left: 0pt, rest: 8pt)),
  h(0.2cm),
	square(radius: (top-left: 0pt, rest: 100pt)),
)

// Join between solid and dotted strokes
#set square(stroke: (left: green + 4pt, top: black + 2pt, right: (paint: blue, dash: "dotted"), bottom: (paint: black, dash: "dotted")))
#stack(
  dir: ltr,
  square(),
  h(0.2cm),
  square(radius: (top-left: 0pt, rest: 1pt)),
  h(0.2cm),
  square(radius: (top-left: 0pt, rest: 8pt)),
  h(0.2cm),
	square(radius: (top-left: 0pt, rest: 100pt)),
)

--- issue-3700-deformed-stroke ---
// Test shape fill & stroke for specific values that used to make the stroke
// deformed.
#rect(
  radius: 1mm,
  width: 100%,
  height: 10pt,
  stroke: (left: rgb("46b3c2") + 16.0mm),
)
