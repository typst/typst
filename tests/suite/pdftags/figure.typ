--- figure-tags-image-basic pdftags pdfstandard(ua-1) ---
// The image is automatically wrapped in a figure tag.
#image(alt: "A tiger", "/assets/images/tiger.jpg")

--- figure-tags-image-figure-with-caption pdftags pdfstandard(ua-1) ---
#figure(
  // The image doesn't create a duplicate figure tag
  // and the alt description is used in the outer figure.
  image(alt: "A tiger", "/assets/images/tiger.jpg"),
  // The caption is stored outside of the figure tag
  // grouped in a nonstruct.
  caption: [Some caption]
)

--- figure-tags-inline-equation-with-caption pdftags pdfstandard(ua-1) ---
#figure(
  math.equation(
    alt: "The Pythagorean theorem: a squared plus b squared is c squared",
    $a^2 + b^2 = c^2$,
  ),
  caption: [Some caption]
)

--- figure-tags-block-equation-with-caption pdftags pdfstandard(ua-1) ---
#figure(
  // The alt description is used in the outer figure.
  math.equation(
    block: true,
    alt: "The Pythagorean theorem: a squared plus b squared is c squared",
    $
      a^2 + b^2 = c^2
    $,
  ),
  caption: [Some caption]
)

--- figure-tags-with-alt-flatten-content-basic pdftags pdfstandard(ua-1) ---
// The inner tags are flattened
#figure(alt: "alt text")[
  $a^2 + b^2 = c^2$

  $sum_(i=1)^n(i)$
]

--- figure-tags-with-alt-flatten-content-complex pdftags pdfstandard(ua-1) ---
#figure(alt: "alt text")[
  #table(
    columns: 2,
    // The link tag needs to be retained
    link("https://github.com/typst/typst")[
      #image("/assets/images/tiger.jpg")
    ],
    image("/assets/images/tiger.jpg"),
    [Some more text],
  )
]

--- figure-tags-alt-with-different-lang pdftags pdfstandard(ua-1) ---
#set text(lang: "de")
Ein Paragraph.

#set text(lang: "en", region: "uk")
#figure(image(alt: "A tiger", "/assets/images/tiger.jpg"))

--- figure-tags-listing pdftags pdfstandard(ua-1) ---
#figure[
  ```rs
  fn main() {
      println!("Hello Typst!");
  }
  ```
]

--- figure-tags-only-marked-content-missing-alt pdftags pdfstandard(ua-1) ---
// Error: 2-3:2 PDF/UA-1 error: missing alt text
// Hint: 2-3:2 make sure your images and equations have alt text
#figure[
  #rect(fill: red)
]

--- figure-tags-only-marked-content pdftags ---
#figure[
  #rect(fill: red)
]

--- figure-tags-additional-caption-inside-body pdftags ---
#figure(caption: [The real caption])[
  #image(alt: "A tiger", "/assets/images/tiger.jpg"),
  #figure.caption[Additional caption]
]

--- figure-tags-additional-caption-inside-table pdftags pdfstandard(ua-1) ---
// Error: 19-35 PDF/UA-1 error: invalid table (Table) structure
// Hint: 19-35 table (Table) may not contain multiple caption (Caption) tags
// Hint: 19-35 avoid manually calling `figure.caption`
#figure(caption: [The real caption])[
  #table(
    columns: 2,
    [A], [B],
    [C], [D],
  )
// Error: 4-38 PDF/UA-1 error: invalid table (Table) structure
// Hint: 4-38 table (Table) may not contain multiple caption (Caption) tags
// Hint: 4-38 avoid manually calling `figure.caption`
  #figure.caption[Additional caption]
]
