// Defines functions for showing figures in the documentation.

#import "base.typ": constrain
#import "example.typ": boxy-block

// Displays a figure in the documentation.
#let docs-figure(
  // The name of the image file in typst-dev-assets.
  filename,
  // Alt text for the image. Required.
  alt: none,
  // A rough cross-target specification of how wide the image should be.
  //
  // In the HTML version, this correspond to pixels. In the paged version it's
  // multiplied with 0.75pt to define a max width.
  width: none,
  // Whether to render a shadow in the web documentation (or a stroke in the
  // paged one). Will typically be the case, but some things like screenshots
  // may come with their own baked shadow.
  shadow: true,
) = {
  assert.ne(alt, none, message: "alt text is required")
  let img = image(docs.read-dev-asset(filename))
  context if target() == "paged" {
    set align(center)
    let ratio = 0.75pt
    show: rest => if width != none {
      constrain(width: width * ratio, rest)
    } else {
      rest
    }

    if shadow {
      figure(boxy-block(img))
    } else {
      figure(img)
    }
  } else {
    html.figure(
      ..if shadow { (class: "shadow") },
      ..if width != none { (style: "width: " + str(width) + "px") },
      img,
    )
  }
}

// Displays two figures side-by-side.
#let side-by-side(l, r) = context {
  if target() == "paged" {
    grid(
      columns: (1fr, 1fr),
      gutter: 0.5em,
      l, r,
    )
  } else {
    html.div(class: "side-by-side", l + r)
  }
}
