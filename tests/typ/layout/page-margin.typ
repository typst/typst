// Test page margins.

---
// Set all margins at once.
#[
  #set page(height: 20pt, margin: 5pt)
  #place(top + left)[TL]
  #place(bottom + right)[BR]
]

---
// Set individual margins.
#set page(height: 40pt)
#[#set page(margin: (left: 0pt)); #align(left)[Left]]
#[#set page(margin: (right: 0pt)); #align(right)[Right]]
#[#set page(margin: (top: 0pt)); #align(top)[Top]]
#[#set page(margin: (bottom: 0pt)); #align(bottom)[Bottom]]

// Ensure that specific margins override general margins.
#[#set page(margin: (rest: 0pt, left: 20pt)); Overridden]
