// Test configuring page sizes and margins.

// Set width and height.
[page width: 120pt, height: 120pt]
[page width: 40pt][High]
[page height: 40pt][Wide]

// Set all margins at once.
[page margins: 30pt][
    [align top, left][TL]
    [align bottom, right][BR]
]

// Set individual margins.
[page height: 40pt]
[page left: 0pt   | align left][Left]
[page right: 0pt  | align right][Right]
[page top: 0pt    | align top][Top]
[page bottom: 0pt | align bottom][Bottom]

// Ensure that specific margins override general margins.
[page margins: 0pt, left: 20pt][Overriden]

// Error: 1:7-1:18 unknown variable
[page nonexistant]

// Error: 1:17-1:20 aligned axis
[page main-dir: ltr]

// Flipped predefined paper.
[page "a11", flip: true][Flipped A11]

// Flipped custom page size.
[page width: 40pt, height: 120pt]
[page flip: true]
Wide

// Test changing the layouting directions of pages.

[page height: 50pt, main-dir: btt, cross-dir: rtl]
Right to left!

---
// Test a combination of pages with bodies and normal content.

[page height: 50pt]

[page][First]
[page][Second]
[pagebreak]
Fourth
[page][]
Sixth
[page][Seventh and last]
