// Test pattern with different `spacing`.

---
// Test with spacing set to `(-10pt, -10pt)`
#let pat(..args) = pattern(size: (30pt, 30pt), ..args)[
  #square(width: 100%, height: 100%, stroke: 1pt, fill: blue)
]

#set page(width: 100pt, height: 100pt)

#rect(fill: pat(spacing: (-10pt, -10pt)), width: 100%, height: 100%, stroke: 1pt)

---
// Test with spacing set to `(0pt, 0pt)`
#let pat(..args) = pattern(size: (30pt, 30pt), ..args)[
  #square(width: 100%, height: 100%, stroke: 1pt, fill: blue)
]

#set page(width: 100pt, height: 100pt)

#rect(fill: pat(spacing: (0pt, 0pt)), width: 100%, height: 100%, stroke: 1pt)

---
// Test with spacing set to `(10pt, 10pt)`
#let pat(..args) = pattern(size: (30pt, 30pt), ..args)[
  #square(width: 100%, height: 100%, stroke: 1pt, fill: blue)
]

#set page(width: 100pt, height: 100pt)

#rect(fill: pat(spacing: (10pt, 10pt,)), width: 100%, height: 100%, stroke: 1pt)
