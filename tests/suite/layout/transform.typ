// Test transformations.

--- transform-tex-logo ---
// Test creating the TeX and XeTeX logos.
#let size = 11pt
#let tex = {
  [T]
  h(-0.14 * size)
  box(move(dy: 0.22 * size)[E])
  h(-0.12 * size)
  [X]
}

#let xetex = {
  [X]
  h(-0.14 * size)
  box(scale(x: -100%, move(dy: 0.26 * size)[E]))
  h(-0.14 * size)
  [T]
  h(-0.14 * size)
  box(move(dy: 0.26 * size)[E])
  h(-0.12 * size)
  [X]
}

#set text(font: "New Computer Modern", size)
Neither #tex, \
nor #xetex!

--- transform-rotate-and-scale ---
// Test combination of scaling and rotation.
#set page(height: 80pt)
#align(center + horizon,
  rotate(20deg, scale(70%, image("/assets/images/tiger.jpg")))
)

--- transform-rotate-origin ---
// Test setting rotation origin.
#rotate(10deg, origin: top + left,
  image("/assets/images/tiger.jpg", width: 50%)
)

--- transform-scale-origin ---
// Test setting scaling origin.
#let r = rect(width: 100pt, height: 10pt, fill: forest)
#set page(height: 65pt)
#box(scale(r, x: 50%, y: 200%, origin: left + top))
#box(scale(r, x: 50%, origin: center))
#box(scale(r, x: 50%, y: 200%, origin: right + bottom))

--- transform-rotate ---
// Test that rotation impact layout.
#set page(width: 200pt)
#set rotate(reflow: true)

#let one(angle) = box(fill: aqua, rotate(angle)[Test Text])
#for angle in range(0, 360, step: 15) {
  one(angle * 1deg)
}

--- transform-rotate-relative-sizing ---
// Test relative sizing in rotated boxes.
#set page(width: 200pt, height: 200pt)
#set text(size: 32pt)
#let rotated(body) = box(rotate(
  90deg,
  box(stroke: 0.5pt, height: 20%, clip: true, body)
))

#set rotate(reflow: false)
Hello #rotated[World]!\

#set rotate(reflow: true)
Hello #rotated[World]!

--- transform-scale ---
// Test that scaling impacts layout.
#set page(width: 200pt)
#set text(size: 32pt)
#let scaled(body) = box(scale(
  x: 20%,
  y: 40%,
  body
))

#set scale(reflow: false)
Hello #scaled[World]!

#set scale(reflow: true)
Hello #scaled[World]!

--- transform-scale-relative-sizing ---
// Test relative sizing in scaled boxes.
#set page(width: 200pt, height: 200pt)
#set text(size: 32pt)
#let scaled(body) = box(scale(
  x: 60%,
  y: 40%,
  box(stroke: 0.5pt, width: 30%, clip: true, body)
))

#set scale(reflow: false)
Hello #scaled[World]!\

#set scale(reflow: true)
Hello #scaled[World]!

--- transform-scale-abs-and-auto ---
// Test scaling by absolute lengths and auto.
#set page(width: 200pt, height: 200pt)
#let cylinder = image("/assets/images/cylinder.svg")

#cylinder
#scale(x: 100pt, y: 50pt, reflow: true, cylinder)
#scale(x: auto, y: 50pt, reflow: true, cylinder)
#scale(x: 100pt, y: auto, reflow: true, cylinder)
#scale(x: 150%, y: auto, reflow: true, cylinder)

--- transform-skew ---
// Test skewing along one axis.
#set page(width: 100pt, height: 60pt)
#set text(size: 12pt)
#let skewed(body) = box(skew(ax: -30deg, body))

#set skew(reflow: false)
Hello #skewed[World]!

#set skew(reflow: true)
Hello #skewed[World]!

--- transform-skew-both-axes ---
// Test skewing along both axes.
#set page(width: 100pt, height: 250pt)
#set text(size: 12pt)
#let skewed(angle) = box(skew(ax: 30deg, ay: angle)[Some Text])

#set skew(reflow: true)
#for angle in range(-30, 31, step: 10) {
  skewed(angle * 1deg)
}

--- transform-skew-origin ---
// Test setting skewing origin.
#set page(width: 100pt, height:40pt)
#set text(spacing: 20pt)
#let square = square.with(width: 8pt)
#let skew-square(origin) = box(place(square(stroke: gray))
  + place(skew(ax: -30deg, ay: -30deg, origin: origin, square())))
#skew-square(center+horizon)
#skew-square(bottom+left)
#skew-square(top+right)
#skew-square(horizon+right)

--- transform-skew-relative-sizing ---
// Test relative sizing in skewed boxes.
#set page(width: 100pt, height: 60pt)
#set text(size: 12pt)
#let skewed(body) = box(skew(
  ax: 30deg,
  box(stroke: 0.5pt, width: 30%, clip: true, body)
))

#set skew(reflow: false)
Hello #skewed[World]!\

#set skew(reflow: true)
Hello #skewed[World]!

--- transform-combinations ---
#block[
  #rect(width: 10pt, height: 10pt)
  #block(inset: 10pt)[
    #rect(width: 10pt, height: 10pt)
    #rotate(45deg, block(inset: 10pt)[
      #block(inset: 10pt)[
        #rect(width: 10pt, height: 10pt)
        Hello world
        #rect(width: 10pt, height: 10pt, radius: 10pt)
        #rotate(45deg, block(inset: 10pt)[
          #rect(width: 10pt, height: 10pt, radius: 10pt)
          #rect(width: 10pt, height: 10pt, radius: 10pt)
        ])
      ]
    ])
  ]
]
