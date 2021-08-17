// Test the `square` function.

---
Auto-sized square. \
#square(fill: eastern)[
  #font(fill: white, weight: bold)
  #align(center)
  #pad(5pt)[Typst]
]

---
// Test height overflow.
#page(width: 75pt, height: 100pt)
#square(fill: conifer)[
  But, soft! what light through yonder window breaks?
]

---
// Test width overflow.
#page(width: 100pt, height: 75pt)
#square(fill: conifer)[
  But, soft! what light through yonder window breaks?
]

---
// Length wins over width and height.
// Error: 09-20 unexpected argument
#square(width: 10cm, height: 20cm, length: 1cm, fill: rgb("eb5278"))
