// Test the `circle` function.

---
// Test auto sizing.

Auto-sized circle. \
#circle(fill: #eb5278, align(center, center, [But, soft!]))

Center-aligned rect in auto-sized circle.
#circle(fill: #43a127)[
    #align(center, center)
    #rect(fill: #9feb52, pad(5pt)[But, soft!])
]

100%-width rect in auto-sized circle. \
#circle(fill: #43a127, rect(width: 100%, fill: #9feb52)[
    But, soft! what light through yonder window breaks?
])

Expanded by height.
#circle(fill: #9feb52)[A \ B \ C]

---
// Test relative sizing.
#rect(width: 100%, height: 50pt, fill: #aaa)[
    #align(center, center)
    #font(color: #fff)
    #circle(radius: 10pt, fill: #239DAD)[A]
    #circle(height: 60%, fill: #239DAD)[B]
    #circle(width: 20% + 20pt, fill: #239DAD)[C]
]

---
// Radius wins over width and height.
// Error: 2:23-2:34 unexpected argument
// Error: 1:36-1:49 unexpected argument
#circle(radius: 10pt, width: 50pt, height: 100pt, fill: #239DAD)

// Width wins over height.
// Error: 22-34 unexpected argument
#circle(width: 20pt, height: 50pt, fill: #239DAD)
