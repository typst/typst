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
// Error: 2:4-2:42 PDF/UA-1 error: PDF artifacts may not contain links
// Hint: 2:4-2:42 references, citations, and footnotes are also considered links in PDF
#pdf.artifact[
  #link("https://github.com/typst/typst")
]

--- link-tags-reference-in-artifact pdftags pdfstandard(ua-1) ---
// Error: 4:3-4:11 PDF/UA-1 error: PDF artifacts may not contain links
// Hint: 4:3-4:11 references, citations, and footnotes are also considered links in PDF
#set heading(numbering: "1.")
= Heading <heading>
#pdf.artifact[
  @heading
]

--- link-tags-citation-in-artifact pdftags pdfstandard(ua-1) ---
// Error: 2:3-2:10 PDF/UA-1 error: PDF artifacts may not contain links
// Hint: 2:3-2:10 references, citations, and footnotes are also considered links in PDF
#pdf.artifact[
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
