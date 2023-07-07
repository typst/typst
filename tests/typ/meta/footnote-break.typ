// Test footnotes that break across pages.

---
#set page(height: 200pt)

#lorem(5)
#footnote[ // 1
  A simple footnote.
  #footnote[Well, not that simple ...] // 2
]
#lorem(15)
#footnote[Another footnote: #lorem(30)] // 3
#lorem(15)
#footnote[My fourth footnote: #lorem(50)] // 4
#lorem(15)
#footnote[And a final footnote.] // 5
