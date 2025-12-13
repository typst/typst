--- list-tags-basic pdftags pdfstandard(ua-1) ---
- a
  - 1
- b
  - c
    - d
  - e
- f

--- list-tags-mixed-with-enum pdftags pdfstandard(ua-1) ---
- a
  + 1
- b
  + c
    - d
  + e
- f

--- list-tags-wide-with-nested-list pdftags pdfstandard(ua-1) ---
- a

  - 1

- b

  - c

    - d

  - e

- f

--- list-tags-complex-item-with-sub-list pdftags pdfstandard(ua-1) ---
- #[#quote(block: true)[hi] #footnote[1].]
  - a
  - b
- c
- d

--- list-tags-complex-item-with-nested-list pdftags pdfstandard(ua-1) ---
- #[
    #quote(block: true)[hi]
    #footnote[1].
    - a
    - b
  ]
- c
- d

--- list-tags-terms-basic pdftags pdfstandard(ua-1) ---
/ A: 1
/ B: 2
/ C: 3

--- list-tags-terms-basic-wide pdftags pdfstandard(ua-1) ---
/ A: 1

/ B: 2

/ C: 3

--- list-tags-terms-indented pdftags pdfstandard(ua-1) ---
/ A: 1
/ B: 2
  / B1: wow
  / B2: amazing

--- list-tags-terms-body-with-parbreak pdftags pdfstandard(ua-1) ---
/ A: 1 #parbreak() 232
/ B: 2

--- list-tags-terms-label-with-parbreak-error pdftags pdfstandard(ua-1) ---
// Error: 1-21 PDF/UA-1 error: invalid document structure, this element's PDF tag would be split up
// Hint: 1-21 this is probably caused by paragraph grouping
// Hint: 1-21 maybe you've used a `parbreak`, `colbreak`, or `pagebreak`
// TODO: This should have the span of the term label, not the entire term item
/ A #parbreak() A: 1
/ B: 2

--- list-tags-terms-label-with-parbreak pdftags ---
// This currently produces an empty paragraph, because terms label is moved out
// of the broken paragraph when constructing the PDF list structure. This only
// happens when tags are broken up, so it's not *that* bad.
/ A #parbreak() A: 1
/ B: 2
