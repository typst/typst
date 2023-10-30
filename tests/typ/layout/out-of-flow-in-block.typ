// Test out-of-flow items (place, counter updates, etc.) at the
// beginning of a block not creating a frame just for them.

---
// No item in the first frame.
#set page(height: 5cm, margin: 1cm)
Not enough space on this page.
#block(breakable: true, stroke: 1pt, inset: 0.5cm)[
  #rect(height: 2cm, fill: gray)
]

---
// Counter update in the first frame.
#set page(height: 5cm, margin: 1cm)
Still not enough space.
#block(breakable: true, stroke: 1pt, inset: 0.5cm)[
  #counter("dummy").step()
  #rect(height: 2cm, fill: gray)
]

---
// Placed item in the first frame.
#set page(height: 5cm, margin: 1cm)
Yet again not enough space.
#block(breakable: true, above: 1cm, stroke: 1pt, inset: 0.5cm)[
  #place(dx: -0.5cm, dy: -0.75cm, box(width: 200%)[OOF])
  #rect(height: 2cm, fill: gray)
]

---
// In-flow item with height zero in the first frame.
#set page(height: 5cm, margin: 1cm)
Guess what, not enough space!
#block(breakable: true, stroke: 1pt, inset: 0.5cm)[
  #set block(spacing: 0pt)
  #line(length: 100%)
  #rect(height: 2cm, fill: gray)
  #line(length: 100%)
]

---
// Counter update and placed item in the first frame.
#set page(height: 5cm, margin: 1cm)
Do I still have to write it?
#block(breakable: true, above: 1cm, stroke: 1pt, inset: 0.5cm)[
  #counter("dummy").step()
  #place(dx: -0.5cm, dy: -0.75cm, box([OOF]))
  #rect(height: 2cm, fill: gray)
]

---
// Mix-and-match all the previous ones.
#set page(height: 5cm, margin: 1cm)
You know where this is going.
#block(breakable: true, above: 1cm, stroke: 1pt, inset: 0.5cm)[
  #counter("dummy").step()
  #place(dx: -0.5cm, dy: -0.75cm, box(width: 200%)[OOF])
  #line(length: 100%)
  #place(dy: -0.8em)[OOF]
  #rect(height: 2cm, fill: gray)
]
