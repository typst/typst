// Test weak page breaks.

---
// Test weak pagebreak after page counter change.
// Should result in two pages.
#set page(numbering: "i")
First
#pagebreak(weak: true)
#set page(numbering: "1")
#counter(page).update(1)
#pagebreak(weak: true)
Second
