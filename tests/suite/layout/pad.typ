// Test the `pad` function.

--- pad-basic paged ---
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

--- pad-expanding-contents paged ---
// Pad can grow.
#pad(left: 10pt, right: 10pt)[PL #h(1fr) PR]

--- pad-followed-by-content paged ---
// Test that the pad element doesn't consume the whole region.
#set page(height: 6cm)
#align(left)[Before]
#pad(10pt, image("/assets/images/tiger.jpg"))
#align(right)[After]

--- pad-adding-to-100-percent paged ---
// Test that padding adding up to 100% does not panic.
#pad(50%)[]

--- issue-5044-pad-100-percent paged ---
#set page(width: 30pt, height: 30pt)
#pad(100%, block(width: 1cm, height: 1cm, fill: red))

--- issue-5160-unbreakable-pad paged ---
#set block(breakable: false)
#block(width: 100%, pad(x: 20pt, align(right)[A]))
