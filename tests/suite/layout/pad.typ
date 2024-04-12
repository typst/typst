// Test the `pad` function.

--- pad-basic ---
// Use for indentation.
#pad(left: 10pt, [Indented!])

// All sides together.
#set rect(inset: 0pt)
#rect(fill: conifer,
  pad(10pt, right: 20pt,
    rect(width: 20pt, height: 20pt, fill: rgb("eb5278"))
  )
)

Hi #box(pad(left: 10pt)[A]) there

--- pad-expanding-contents ---
// Pad can grow.
#pad(left: 10pt, right: 10pt)[PL #h(1fr) PR]

--- pad-followed-by-content ---
// Test that the pad element doesn't consume the whole region.
#set page(height: 6cm)
#align(left)[Before]
#pad(10pt, image("/assets/images/tiger.jpg"))
#align(right)[After]

--- pad-adding-to-100-percent ---
// Test that padding adding up to 100% does not panic.
#pad(50%)[]
