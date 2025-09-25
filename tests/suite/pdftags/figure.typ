--- figure-tags-image-basic pdftags ---
// The image is automatically wrapped in a figure tag.
#image(alt: "A tiger", "/assets/images/tiger.jpg")

--- figure-tags-image-figure-with-caption pdftags ---
#figure(
  // The image doesn't create a duplicate figure tag
  // and the alt description is used in the outer figure.
  image(alt: "A tiger", "/assets/images/tiger.jpg"),
  // The caption is stored outside of the figure tag
  // grouped in a nonstruct.
  caption: [Some caption]
)

--- figure-tags-inline-equation-with-caption pdftags ---
#figure(
  math.equation(
    alt: "The Pythagorean theorem: a squared plus b squared is c squared",
    $a^2 + b^2 = c^2$,
  ),
  caption: [Some caption]
)

--- figure-tags-block-equation-with-caption pdftags ---
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
