--- flow-fr render ---
#set page(height: 2cm)
#set text(white)
#rect(fill: forest)[
          #v(1fr)
  #h(1fr) Hi you!
]

--- issue-flow-overlarge-frames render ---
// In this bug, the first line of the second paragraph was on its page alone an
// the rest moved down. The reason was that the second block resulted in
// overlarge frames because the region wasn't finished properly.
#set page(height: 70pt)
#block(lines(3))
#block(lines(5))

--- issue-flow-trailing-leading render ---
// In this bug, the first part of the paragraph moved down to the second page
// because trailing leading wasn't trimmed, resulting in an overlarge frame.
#set page(height: 60pt)
#v(19pt)
#block[
  But, soft! what light through yonder window breaks?
  It is the east, and Juliet is the sun.
]

--- issue-flow-weak-spacing render ---
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

--- issue-flow-frame-placement render ---
// In this bug, a frame intended for the second region ended up in the first.
#set page(height: 105pt)
#block(lorem(20))

--- issue-flow-layout-index-out-of-bounds render ---
// This bug caused an index-out-of-bounds panic when layouting paragraphs needed
// multiple reorderings.
#set page(height: 200pt)
#lines(10)

#figure(placement: auto, block(height: 100%))

#lines(3)

#lines(3)

--- issue-3641-float-loop render ---
// Flow layout should terminate!
#set page(height: 40pt)

= Heading
#lines(2)

--- issue-3355-metadata-weak-spacing render ---
#set page(height: 50pt)
#block(width: 100%, height: 30pt, fill: aqua)
#metadata(none)
#v(10pt, weak: true)
Hi

--- issue-3866-block-migration render ---
#set page(height: 120pt)
#set text(costs: (widow: 0%, orphan: 0%))
#v(50pt)
#columns(2)[
  #lines(6)
  #block(rect(width: 80%, height: 80pt), breakable: false)
  #lines(6)
]

--- issue-5024-spill-backlog render ---
#set page(columns: 2, height: 50pt)
#columns(2)[Hello]
