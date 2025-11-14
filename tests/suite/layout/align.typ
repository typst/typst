// Test alignment.

--- align-right paged ---
// Test ragged-left.
#set align(right)
To the right! Where the sunlight peeks behind the mountain.

--- align-in-stack paged ---
#set page(height: 100pt)
#stack(dir: ltr,
  align(left, square(size: 15pt, fill: eastern)),
  align(center, square(size: 20pt, fill: eastern)),
  align(right, square(size: 15pt, fill: eastern)),
)
#align(center + horizon, rect(fill: eastern, height: 10pt))
#align(bottom, stack(
  align(center, rect(fill: conifer, height: 10pt)),
  rect(fill: forest, height: 10pt, width: 100%),
))

--- align-center-in-flow paged ---
// Test that multiple paragraphs in subflow also respect alignment.
#align(center)[
  Lorem Ipsum

  Dolor
]

--- align-start-and-end paged ---
// Test start and end alignment.
#rotate(-30deg, origin: end + horizon)[Hello]

#set text(lang: "de")
#align(start)[Start]
#align(end)[Ende]

#set text(lang: "ar", font: "Noto Sans Arabic")
#align(start)[يبدأ]
#align(end)[نهاية]

--- alignment-fields-x paged ---
// Test 2d alignment 'horizontal' field.
#test((start + top).x, start)
#test((end + top).x, end)
#test((left + top).x, left)
#test((right + top).x, right)
#test((center + top).x, center)
#test((start + bottom).x, start)
#test((end + bottom).x, end)
#test((left + bottom).x, left)
#test((right + bottom).x, right)
#test((center + bottom).x, center)
#test((start + horizon).x, start)
#test((end + horizon).x, end)
#test((left + horizon).x, left)
#test((right + horizon).x, right)
#test((center + horizon).x, center)
#test((top + start).x, start)
#test((bottom + end).x, end)
#test((horizon + center).x, center)

--- alignment-fields-y paged ---
// Test 2d alignment 'vertical' field.
#test((start + top).y, top)
#test((end + top).y, top)
#test((left + top).y, top)
#test((right + top).y, top)
#test((center + top).y, top)
#test((start + bottom).y, bottom)
#test((end + bottom).y, bottom)
#test((left + bottom).y, bottom)
#test((right + bottom).y, bottom)
#test((center + bottom).y, bottom)
#test((start + horizon).y, horizon)
#test((end + horizon).y, horizon)
#test((left + horizon).y, horizon)
#test((right + horizon).y, horizon)
#test((center + horizon).y, horizon)
#test((top + start).y, top)
#test((bottom + end).y, bottom)
#test((horizon + center).y, horizon)

--- alignment-type paged ---
#test(type(center), alignment)
#test(type(horizon), alignment)
#test(type(center + horizon), alignment)

--- alignment-axis paged ---
// Test alignment methods.
#test(start.axis(), "horizontal")
#test(end.axis(), "horizontal")
#test(left.axis(), "horizontal")
#test(right.axis(), "horizontal")
#test(center.axis(), "horizontal")
#test(top.axis(), "vertical")
#test(bottom.axis(), "vertical")
#test(horizon.axis(), "vertical")

--- alignment-inv paged ---
#test(start.inv(), end)
#test(end.inv(), start)
#test(left.inv(), right)
#test(right.inv(), left)
#test(center.inv(), center)
#test(top.inv(), bottom)
#test(bottom.inv(), top)
#test(horizon.inv(), horizon)
#test((start + top).inv(), (end + bottom))
#test((end + top).inv(), (start + bottom))
#test((left + top).inv(), (right + bottom))
#test((right + top).inv(), (left + bottom))
#test((center + top).inv(), (center + bottom))
#test((start + bottom).inv(), (end + top))
#test((end + bottom).inv(), (start + top))
#test((left + bottom).inv(), (right + top))
#test((right + bottom).inv(), (left + top))
#test((center + bottom).inv(), (center + top))
#test((start + horizon).inv(), (end + horizon))
#test((end + horizon).inv(), (start + horizon))
#test((left + horizon).inv(), (right + horizon))
#test((right + horizon).inv(), (left + horizon))
#test((center + horizon).inv(), (center + horizon))
#test((top + start).inv(), (end + bottom))
#test((bottom + end).inv(), (start + top))
#test((horizon + center).inv(), (center + horizon))

--- alignment-add-two-horizontal paged ---
// Error: 8-22 cannot add two horizontal alignments
#align(center + right, [A])

--- alignment-add-two-vertical paged ---
// Error: 8-20 cannot add two vertical alignments
#align(top + bottom, [A])

--- alignment-add-vertical-and-2d paged ---
// Error: 8-30 cannot add a vertical and a 2D alignment
#align(top + (bottom + right), [A])

--- issue-1398-line-align paged ---
// Test right-aligning a line and a rectangle.
#align(right, line(length: 30%))
#align(right, rect())

--- issue-2213-align-fr paged ---
// Test a mix of alignment and fr units (fr wins).
#set page(height: 80pt)
A
#v(1fr)
B
#align(bottom + right)[C]
