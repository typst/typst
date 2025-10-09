--- logical-children-tags-place-within-artifact pdftags ---
// Error: 2:4-4:4 PDF/UA-1 error: PDF artifacts may not contain links
// Hint: 2:4-4:4 references, citations, and footnotes are also considered links in PDF
#pdf.artifact[
  #link("tel:123")[
    #place(float: true, top + left, rect(fill: red))
  ]
]

--- logical-children-tags-link-within-place-within-artifact pdftags ---
// Error: 3:6-5:6 PDF/UA-1 error: PDF artifacts may not contain links
// Hint: 3:6-5:6 references, citations, and footnotes are also considered links in PDF
#pdf.artifact[
  #place(float: true, top + left)[
    #link("tel:123")[
      #rect(fill: red)
    ]
  ]
]

--- logical-children-tags-footnote-in-tiling pdftags ---
// Error: PDF/UA-1 error: PDF artifacts may not contain links
// Hint: a link was used within a tiling
// Hint: references, citations, and footnotes are also considered links in PDF
#rect(width: 90pt, height: 90pt, fill: tiling(size: (30pt, 30pt))[
  #footnote[hi]
])

--- logical-children-tags-place-in-tiling pdftags ---
#rect(width: 90pt, height: 90pt, fill: tiling(size: (30pt, 30pt))[
  #place(float: true, top + right)[hi]
])
