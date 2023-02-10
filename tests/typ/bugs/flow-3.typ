// In this bug, there was a bit of space below the heading because weak spacing
// directly before a layout-induced column or page break wasn't trimmed.

---
#set page(height: 60pt)
#rect(inset: 0pt, columns(2)[
  Text
  #v(12pt)
  Hi
  #v(10pt, weak: true)
  At column break.
])
