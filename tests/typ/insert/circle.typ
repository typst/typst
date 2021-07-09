// Test the `circle` function.

---
// Test auto sizing.

Auto-sized circle. \
#circle(fill: rgb("eb5278"))[
    #align!(center, center)
    But, soft!
]

Center-aligned rect in auto-sized circle.
#circle(fill: forest)[
    #align!(center, center)
    #rect!(fill: conifer)
    #pad!(5pt)
    But, soft!
]

100%-width rect in auto-sized circle. \
#circle(fill: forest)[
    #rect!(width: 100%, fill: conifer)
    But, soft! what light through yonder window breaks?
]

Expanded by height.
#circle(fill: conifer)[A \ B \ C]

---
// Test relative sizing.
#rect(width: 100%, height: 50pt, fill: rgb("aaa"))[
    #align!(center, center)
    #font!(fill: white)
    #circle(radius: 10pt, fill: eastern)[A]
    #circle(height: 60%, fill: eastern)[B]
    #circle(width: 20% + 20pt, fill: eastern)[C]
]

---
// Radius wins over width and height.
// Error: 23-34 unexpected argument
// Error: 36-49 unexpected argument
#circle(radius: 10pt, width: 50pt, height: 100pt, fill: eastern)

// Width wins over height.
// Error: 22-34 unexpected argument
#circle(width: 20pt, height: 50pt, fill: eastern)
