// Test that the logic that keeps footnote entry together with
// their markers also works for multiple footnotes in a single
// line or frame (here, there are two lines, but they are one
// unit due to orphan prevention).

---
#set page(height: 100pt)
#v(40pt)
A #footnote[a] \
B #footnote[b]
