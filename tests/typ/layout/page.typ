// Test configuring page sizes and margins.

---
// Set width and height.
#page(width: 120pt, height: 120pt)
#page(width: 40pt)[High]
#page(height: 40pt)[Wide]

// Set all margins at once.
#page(margins: 30pt)[
    #align(top, left)[TL]
    #align(bottom, right)[BR]
]

// Set individual margins.
#page(height: 40pt)
#page(left: 0pt, align(left)[Left])
#page(right: 0pt, align(right)[Right])
#page(top: 0pt, align(top)[Top])
#page(bottom: 0pt, align(bottom)[Bottom])

// Ensure that specific margins override general margins.
#page(margins: 0pt, left: 20pt)[Overriden]

// Error: 7-18 unknown variable
#page(nonexistant)

// Flipped predefined paper.
#page("a11", flip: true)[Flipped A11]

// Flipped custom page size.
#page(width: 40pt, height: 120pt)
#page(flip: true)
Wide

---
// Test a combination of pages with bodies and normal content.

#page(height: 50pt)

#page[First]
#page[Second]
#pagebreak()
#pagebreak()
Fourth
#page[]
Sixth
#page[Seventh and last]
