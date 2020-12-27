// Test configuring page sizes and margins.

// Set width.
[page width: 50pt][High]

// Set height.
[page height: 50pt][Wide]

// Set all margins at once.
[page margins: 40pt][
    [align top, left][TL]
    [align bottom, right][BR]
]

// Set individual margins.
[page left: 0pt   | align left][Left]
[page right: 0pt  | align right][Right]
[page top: 0pt    | align top][Top]
[page bottom: 0pt | align bottom][Bottom]

// Ensure that specific margins override general margins.
[page margins: 0pt, left: 40pt][Overriden]

// Flip the page.
[page a10, flip: true][Flipped]
