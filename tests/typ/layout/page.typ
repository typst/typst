// Test configuring page sizes and margins.

---
// Set width and height.
#page(width: 80pt, height: 80pt)
[#page(width: 40pt) High]
[#page(height: 40pt) Wide]

// Set all margins at once.
[
  #page(margins: 5pt)
  #place(top, left)[TL]
  #place(bottom, right)[BR]
]

// Set individual margins.
#page(height: 40pt)
[#page(left: 0pt) #align(left)[Left]]
[#page(right: 0pt) #align(right)[Right]]
[#page(top: 0pt) #align(top)[Top]]
[#page(bottom: 0pt) #align(bottom)[Bottom]]

// Ensure that specific margins override general margins.
[#page(margins: 0pt, left: 20pt) Overriden]

// Flipped predefined paper.
[#page(paper: "a11", flip: true) Flipped A11]

---
// Test a combination of pages with bodies and normal content.

#page(width: 80pt, height: 30pt)

[#page() First]
[#page() Second]
#pagebreak()
#pagebreak()
Fourth
[#page(height: 25pt)]
Sixth
[#page() Seventh]

---
#page(width: 80pt, height: 40pt, fill: eastern)
#font(15pt, "Roboto", fill: white, smallcaps: true)
Typst
