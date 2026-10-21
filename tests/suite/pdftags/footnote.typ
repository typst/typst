--- footnote-tags-basic pdftags ---
#set pdf(standard: "ua-1")
Footnote #footnote[Hi] in text.

--- footnote-tags-different-lang pdftags ---
#set pdf(standard: "ua-1")
Footnote #footnote[
  // The footnote number is still in English ("en"), so the link tag
  // holding the number should specify its language to be English, so
  // as to override the parent tag's language, which is German ("de").
  #set text(lang: "de")
  Hallo
] in text.

--- footnote-tags-ref-to-other-footnote pdftags ---
#set pdf(standard: "ua-1")
This #footnote[content]<note> and #footnote(<note>).
