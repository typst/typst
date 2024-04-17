--- flow-fr ---
#set page(height: 2cm)
#set text(white)
#rect(fill: forest)[
          #v(1fr)
  #h(1fr) Hi you!
]

--- issue-flow-overlarge-frames ---
// In this bug, the first line of the second paragraph was on its page alone an
// the rest moved down. The reason was that the second block resulted in
// overlarge frames because the region wasn't finished properly.
#set page(height: 70pt)
#block[This file tests a bug where an almost empty page occurs.]
#block[
  The text in this second block was torn apart and split up for
  some reason beyond my knowledge.
]

--- issue-flow-trailing-leading ---
// In this bug, the first part of the paragraph moved down to the second page
// because trailing leading wasn't trimmed, resulting in an overlarge frame.
#set page(height: 60pt)
#v(19pt)
#block[
  But, soft! what light through yonder window breaks?
  It is the east, and Juliet is the sun.
]

--- issue-flow-weak-spacing ---
// In this bug, there was a bit of space below the heading because weak spacing
// directly before a layout-induced column or page break wasn't trimmed.
#set page(height: 60pt)
#rect(inset: 0pt, columns(2)[
  Text
  #v(12pt)
  Hi
  #v(10pt, weak: true)
  At column break.
])

--- issue-flow-frame-placement ---
// In this bug, a frame intended for the second region ended up in the first.
#set page(height: 105pt)
#block(lorem(20))

--- issue-flow-layout-index-out-of-bounds ---
// This bug caused an index-out-of-bounds panic when layouting paragraphs needed
// multiple reorderings.
#set page(height: 200pt)
#lorem(30)

#figure(placement: auto, block(height: 100%))

#lorem(10)

#lorem(10)

--- issue-3641-float-loop ---
// Flow layout should terminate!
//
// This is not yet ideal: The heading should not move to the second page, but
// that's a separate bug and not a regression.
#set page(height: 40pt)

= Heading
#lorem(6)
