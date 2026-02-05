--- layout-tags-placement-float pdftags pdfstandard(ua-1) ---
#set text(lang: "de")
#grid(
  columns: 2,
  [
    #set text(lang: "be")
    text
    #place(float: true, top + left)[
      `a`
    ]
  ],
  [
    text
    #place(float: true, top + left)[
      #set text(lang: "fr")
      text in grid
    ]
    text
  ],
  [
    #set text(lang: "es")
    b
  ]
)

--- layout-tags-list-marker-issue-7789 pdftags ---
#set list(marker: [a] + layout(layout_info => box(height: 100em)))

- A
  - A
