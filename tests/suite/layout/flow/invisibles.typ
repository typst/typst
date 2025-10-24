// Test out-of-flow items (place, counter updates, etc.) at the
// beginning of a block not creating a frame just for them.

--- flow-first-region-no-item render ---
// No item in the first region.
#set page(height: 5cm, margin: 1cm)
No item in the first region.
#block(breakable: true, stroke: 1pt, inset: 0.5cm)[
  #rect(height: 2cm, fill: gray)
]

--- flow-first-region-counter-update render ---
// Counter update in the first region.
#set page(height: 5cm, margin: 1cm)
Counter update.
#block(breakable: true, stroke: 1pt, inset: 0.5cm)[
  #counter("dummy").step()
  #rect(height: 2cm, fill: gray)
]

--- flow-first-region-placed render ---
// Placed item in the first region.
#set page(height: 5cm, margin: 1cm)
Placed item in the first region.
#block(breakable: true, above: 1cm, stroke: 1pt, inset: 0.5cm)[
  #place(dx: -0.5cm, dy: -0.75cm, box(width: 200%)[OOF])
  #rect(height: 2cm, fill: gray)
]

--- flow-first-region-zero-sized-item render ---
// In-flow item with size zero in the first region.
#set page(height: 5cm, margin: 1cm)
In-flow, zero-sized item.
#block(breakable: true, stroke: 1pt, inset: 0.4cm)[
  #set block(spacing: 0pt)
  #line(length: 0pt)
  #rect(height: 2cm, fill: gray)
  #line(length: 100%)
]

--- flow-first-region-counter-update-and-placed render ---
// Counter update and placed item in the first region.
#set page(height: 5cm, margin: 1cm)
Counter update + place.
#block(breakable: true, above: 1cm, stroke: 1pt, inset: 0.5cm)[
  #counter("dummy").step()
  #place(dx: -0.5cm, dy: -0.75cm, box([OOF]))
  #rect(height: 2cm, fill: gray)
]

--- flow-first-region-counter-update-placed-and-line render ---
// Mix-and-match all the previous ones.
#set page(height: 5cm, margin: 1cm)
Mix-and-match all the previous tests.
#block(breakable: true, above: 1cm, stroke: 1pt, inset: 0.5cm)[
  #counter("dummy").step()
  #place(dx: -0.5cm, dy: -0.75cm, box(width: 200%)[OOF])
  #line(length: 100%)
  #place(dy: 0.2em)[OOF]
  #rect(height: 2cm, fill: gray)
]
