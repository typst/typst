--- link-tags-heading-without-numbering pdftags pdfstandard(ua-1) ---
= Heading <heading>

#link(<heading>)[link to heading]

--- link-tags-heading-with-numbering pdftags pdfstandard(ua-1) ---
#set heading(numbering: "1.")
= Heading <heading>

#link(<heading>)[link to heading]

--- link-tags-non-refable-location pdftags pdfstandard(ua-1) ---
A random location <somewhere>

#link(<somewhere>)[link to somewhere]

--- link-tags-contact-prefix pdftags pdfstandard(ua-1) ---
#link("mailto:hello@typst.app")

#link("tel:123")

--- link-tags-position pdftags pdfstandard(ua-1) ---
#context link(here().position())[somewhere]

--- link-tags-link-in-artifact pdftags pdfstandard(ua-1) ---
#pdf.artifact[
  // Error: 4-42 PDF/UA-1 error: PDF artifacts may not contain links
  // Hint: 4-42 references, citations, and footnotes are also considered links in PDF
  #link("https://github.com/typst/typst")
]

--- link-tags-reference-in-artifact pdftags pdfstandard(ua-1) ---
#set heading(numbering: "1.")
= Heading <heading>
#pdf.artifact[
  // Error: 3-11 PDF/UA-1 error: PDF artifacts may not contain links
  // Hint: 3-11 references, citations, and footnotes are also considered links in PDF
  @heading
]

--- link-tags-citation-in-artifact pdftags pdfstandard(ua-1) ---
#pdf.artifact[
  // Error: 3-10 PDF/UA-1 error: PDF artifacts may not contain links
  // Hint: 3-10 references, citations, and footnotes are also considered links in PDF
  @netwok
]
#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- link-tags-with-parbreak-error pdftags pdfstandard(ua-1) ---
// Error: 7-69 PDF/UA-1 error: invalid document structure, this element's PDF tag would be split up
// Hint: 7-69 this is probably caused by paragraph grouping
// Hint: 7-69 maybe you've used a `parbreak`, `colbreak`, or `pagebreak`
Look #link("https://github.com/typst/typst")[this #parbreak() thing].

--- link-tags-with-parbreak pdftags ---
Look #link("https://github.com/typst/typst")[this #parbreak() thing].

--- issue-7301-link-tags-empty-link-body pdftags ---
#link("asf")[#none\ #none]

--- issue-7301-link-tags-empty-link-body-linebreak pdftags ---
#link("asf", linebreak())

--- issue-7301-link-tags-empty-link-body-footnote pdftags ---
#footnote(numbering: it => "", [asdf])

--- issue-7301-link-tags-empty-link-body-mutliple pdftags ---
#link("asf")[#none\ #none] #link("asf")[#none\ #none]
