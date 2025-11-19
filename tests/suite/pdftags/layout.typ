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
