--- place-flush ---
#set page(height: 120pt)
#let floater(align, height) = place(
  align,
  float: true,
  rect(width: 100%, height: height),
)

#floater(top, 30pt)
A

#floater(bottom, 50pt)
#place.flush()
B // Should be on the second page.

--- place-flush-figure ---
#set page(height: 120pt)
#let floater(align, height, caption) = figure(
  placement: align,
  caption: caption,
  rect(width: 100%, height: height),
)

#floater(top, 30pt)[I]
A

#floater(bottom, 50pt)[II]
#place.flush()
B // Should be on the second page.
