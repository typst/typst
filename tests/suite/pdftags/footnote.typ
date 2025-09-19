--- footnote-tags-basic pdftags ---
Footnote #footnote[Hi] in text.

--- footnote-tags-different-lang pdftags ---
Footnote #footnote[
  // The footnote number is still in english
  #set text(lang: "de")
  Hallo
] in text.

--- footnote-tags-ref-to-other-footnote pdftags ---
This #footnote[content]<note> and #footnote(<note>).
