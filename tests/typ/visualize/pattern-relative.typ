// Test pattern with different `relative`.

---
// Test with relative set to `"self"`
#let pat(..args) = pattern(size: (30pt, 30pt), ..args)[
  #place(top + left, line(start: (0%, 0%), end: (100%, 100%), stroke: 1pt))
  #place(top + left, line(start: (0%, 100%), end: (100%, 0%), stroke: 1pt))
]

#set page(fill: pat(), width: 100pt, height: 100pt)

#rect(fill: pat(relative: "self"), width: 100%, height: 100%, stroke: 1pt)

---
// Test with relative set to `"parent"`
#let pat(..args) = pattern(size: (30pt, 30pt), ..args)[
  #place(top + left, line(start: (0%, 0%), end: (100%, 100%), stroke: 1pt))
  #place(top + left, line(start: (0%, 100%), end: (100%, 0%), stroke: 1pt))
]

#set page(fill: pat(), width: 100pt, height: 100pt)

#rect(fill: pat(relative: "parent"), width: 100%, height: 100%, stroke: 1pt)
