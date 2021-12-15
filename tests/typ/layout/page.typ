// Test configuring page sizes and margins.

---
// Set width and height.
#set page(width: 80pt, height: 80pt)
[#set page(width: 40pt);High]
[#set page(height: 40pt);Wide]

// Set all margins at once.
[
  #set page(margins: 5pt)
  #place(top + left)[TL]
  #place(bottom + right)[BR]
]

// Set individual margins.
#set page(height: 40pt)
[#set page(left: 0pt); #align(left)[Left]]
[#set page(right: 0pt); #align(right)[Right]]
[#set page(top: 0pt); #align(top)[Top]]
[#set page(bottom: 0pt); #align(bottom)[Bottom]]

// Ensure that specific margins override general margins.
[#set page(margins: 0pt, left: 20pt); Overriden]

// Flipped predefined paper.
[#set page(paper: "a11", flipped: true);Flipped A11]

---
#set page(width: 80pt, height: 40pt, fill: eastern)
#text(15pt, "Roboto", fill: white, smallcaps: true)[Typst]

#set page(width: 40pt, fill: none, margins: auto, top: 10pt)
Hi
