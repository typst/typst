// Test the `square` function.

---
Auto-sized square. \
#square(fill: eastern)[
    #align!(center)
    #pad(5pt, font(color: #fff, weight: bold)[Typst])
]

---
// Length wins over width and height.
// Error: 2:9-2:20 unexpected argument
// Error: 1:22-1:34 unexpected argument
#square(width: 10cm, height: 20cm, length: 1cm, fill: #eb5278)

---
// Test height overflow.
#page!(width: 75pt, height: 100pt)
#square(fill: conifer)[
    But, soft! what light through yonder window breaks?
]

---
// Test width overflow.
#page!(width: 100pt, height: 75pt)
#square(fill: conifer)[
    But, soft! what light through yonder window breaks?
]
