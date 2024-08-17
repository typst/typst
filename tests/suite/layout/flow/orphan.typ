// Test that lines and headings doesn't become orphan.

--- flow-heading-no-orphan ---
#set page(height: 100pt)
#lorem(12)

= Introduction
This is the start and it goes on.

--- flow-par-no-orphan-and-widow-lines ---
// LARGE
#set page("a8", height: 140pt)
#set text(weight: 700)

// Fits fully onto the first page.
#set text(blue)
#lorem(27)

// The first line would fit, but is moved to the second page.
#lorem(20)

// The second-to-last line is moved to the third page so that the last is isn't
// as lonely.
#set text(maroon)
#lorem(11)

#lorem(13)

// All three lines go to the next page.
#set text(olive)
#lorem(10)

--- flow-widow-forced ---
// Ensure that a widow is allowed when the three lines don't all fit.
#set page(height: 50pt)
#lorem(10)

--- issue-1445-widow-orphan-unnecessary-skip ---
// Ensure that widow/orphan prevention doesn't unnecessarily move things
// to another page.
#set page(width: 16cm)
#block(height: 30pt, fill: aqua, columns(2, lorem(19)))
