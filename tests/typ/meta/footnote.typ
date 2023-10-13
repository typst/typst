// Test footnotes.

---
#footnote[Hi]

---
// Test space collapsing before footnote.
A#footnote[A] \
A #footnote[A]

---
// Test nested footnotes.
First \
Second #footnote[A, #footnote[B, #footnote[C]]] \
Third #footnote[D, #footnote[E]] \
Fourth

---
// Currently, numbers a bit out of order if a nested footnote ends up in the
// same frame as another one. :(
#footnote[A, #footnote[B]], #footnote[C]

---
// Test customization.
#show footnote: set text(red)
#show footnote.entry: set text(8pt, style: "italic")
#set footnote.entry(
  indent: 0pt,
  gap: 0.6em,
  clearance: 0.3em,
  separator: repeat[.],
)

Beautiful footnotes. #footnote[Wonderful, aren't they?]
