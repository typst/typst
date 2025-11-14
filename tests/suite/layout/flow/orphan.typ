// Test that lines and headings doesn't become orphan.

--- flow-heading-no-orphan paged ---
#set page(height: 100pt)
#lines(4)

= Introduction
A

--- flow-par-no-orphan-and-widow-lines paged ---
#set page(width: 60pt, height: 140pt)
#set text(weight: 700)

// Fits fully onto the first page.
#set text(blue)
#lines(8)

// The first line would fit, but is moved to the second page.
#lines(6, "1")

// The second-to-last line is moved to the third page so that the last is isn't
// as lonely.
#set text(maroon)
#lines(4)

#lines(4, "1")

// All three lines go to the next page.
#set text(olive)
#lines(3)

--- flow-widow-forced paged ---
// Ensure that a widow is allowed when the three lines don't all fit.
#set page(height: 50pt)
#lines(3)

--- issue-1445-widow-orphan-unnecessary-skip paged ---
// Ensure that widow/orphan prevention doesn't unnecessarily move things
// to another page.
#set page(width: 16cm)
#block(height: 30pt, fill: aqua, columns(2, lorem(19)))
