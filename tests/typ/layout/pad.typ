// Test the `pad` function.

---
// Use for indentation.
#pad(left: 10pt, [Indented!])

// All sides together.
#rect(fill: conifer)[
  #pad(10pt, right: 20pt)[
    #rect(width: 20pt, height: 20pt, fill: #eb5278)
  ]
]

// Error: 14-24 missing argument: body
Hi #rect(pad(left: 10pt)) there

---
// Test that the pad node doesn't consume the whole region.

#page(width: 4cm, height: 5cm)
#align(left)[Before]
#pad(10pt, image("../../res/tiger.jpg"))
#align(right)[After]
