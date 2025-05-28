--- gradient-linear-angled ---
// Test gradients with direction.
#set page(width: 90pt)
#grid(
  gutter: 3pt,
  columns: 4,
  ..range(0, 360, step: 15).map(i => box(
    height: 15pt,
    width: 15pt,
    fill: gradient.linear(angle: i * 1deg, (red, 0%), (blue, 100%)),
  ))
)


--- gradient-linear-oklab ---
// The tests below test whether hue rotation works correctly.
// Here we test in Oklab space for reference.
#set page(
  width: 100pt,
  height: 30pt,
  fill: gradient.linear(red, purple, space: oklab)
)

--- gradient-linear-oklch ---
// Test in OkLCH space.
#set page(
  width: 100pt,
  height: 30pt,
  fill: gradient.linear(red, purple, space: oklch)
)

--- gradient-linear-hsv ---
// Test in HSV space.
#set page(
  width: 100pt,
  height: 30pt,
  fill: gradient.linear(red, purple, space: color.hsv)
)

--- gradient-linear-hsl ---
// Test in HSL space.
#set page(
  width: 100pt,
  height: 30pt,
  fill: gradient.linear(red, purple, space: color.hsl)
)

--- gradient-linear-relative-parent ---
// The image should look as if there is a single gradient that is being used for
// both the page and the rectangles.
#let grad = gradient.linear(red, blue, green, purple, relative: "parent")
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 50pt,
  width: 50pt,
  margin: 2.5pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#place(top + right, my-rect)
#place(bottom + center, rotate(45deg, my-rect))

--- gradient-linear-relative-self ---
// The image should look as if there are multiple gradients, one for each
// rectangle.
#let grad = gradient.linear(red, blue, green, purple, relative: "self")
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 50pt,
  width: 50pt,
  margin: 2.5pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#place(top + right, my-rect)
#place(bottom + center, rotate(45deg, my-rect))

--- gradient-linear-relative-parent-block ---
// The image should look as if there are two nested gradients, one for the page
// and one for a nested block. The rotated rectangles are not visible because
// they are relative to the block.
#let grad = gradient.linear(red, blue, green, purple, relative: "parent")
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 50pt,
  width: 50pt,
  margin: 5pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#block(
  width: 40pt,
  height: 40pt,
  inset: 2.5pt,
  fill: grad,
)[
  #place(top + right, my-rect)
  #place(bottom + center, rotate(45deg, my-rect))
]

--- gradient-linear-repeat-and-mirror-1 ---
// Test repeated gradients.
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.inferno).repeat(2, mirror: true)
)

--- gradient-linear-repeat-and-mirror-2 ---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.rainbow).repeat(2, mirror: true),
)

--- gradient-linear-repeat-and-mirror-3 ---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.rainbow).repeat(5, mirror: true)
)

--- gradient-linear-sharp-and-repeat ---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.rainbow).sharp(10).repeat(5, mirror: false)
)

--- gradient-linear-sharp-repeat-and-mirror ---
#rect(
  height: 40pt,
  width: 100%,
  fill: gradient.linear(..color.map.rainbow).sharp(10).repeat(5, mirror: true)
)

--- gradient-linear-sharp ---
#square(
  size: 100pt,
  fill: gradient.linear(..color.map.rainbow, space: color.hsl).sharp(10),
)
#square(
  size: 100pt,
  fill: gradient.radial(..color.map.rainbow, space: color.hsl).sharp(10),
)
#square(
  size: 100pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsl).sharp(10),
)

--- gradient-linear-sharp-and-smooth ---
#square(
  size: 100pt,
  fill: gradient.linear(..color.map.rainbow, space: color.hsl).sharp(10, smoothness: 40%),
)
#square(
  size: 100pt,
  fill: gradient.radial(..color.map.rainbow, space: color.hsl).sharp(10, smoothness: 40%),
)
#square(
  size: 100pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsl).sharp(10, smoothness: 40%),
)

--- gradient-linear-stroke ---
#align(center + top, square(size: 50pt, fill: black, stroke: 5pt + gradient.linear(red, blue)))

--- gradient-fill-and-stroke ---
#align(
  center + bottom,
  square(
    size: 50pt,
    fill: gradient.radial(red, blue, radius: 70.7%, focal-center: (10%, 10%)),
    stroke: 10pt + gradient.radial(red, blue, radius: 70.7%, focal-center: (10%, 10%))
  )
)

--- gradient-linear-stroke-relative-parent ---
// The image should look as if there is a single gradient that is being used for
// both the circle stroke and the block fill.
#align(
  center + horizon,
  block(
    width: 50pt,
    height: 50pt,
    fill: gradient.linear(red, blue).sharp(4),
    circle(
      radius: 18pt,
      stroke: 5pt + gradient.linear(red, blue, relative: "parent").sharp(4),
    )
  )
)

--- gradient-linear-line ---
// Test gradient on lines
#set page(width: 100pt, height: 100pt)
#line(length: 100%, stroke: 1pt + gradient.linear(red, blue))
#line(length: 100%, angle: 10deg, stroke: 1pt + gradient.linear(red, blue))
#line(length: 100%, angle: 10deg, stroke: 1pt + gradient.linear(red, blue, relative: "parent"))

--- gradient-radial-hsl ---
#square(
  size: 100pt,
  fill: gradient.radial(..color.map.rainbow, space: color.hsl),
)

--- gradient-radial-center ---
#grid(
  columns: 2,
  square(
    size: 50pt,
    fill: gradient.radial(..color.map.rainbow, space: color.hsl, center: (0%, 0%)),
  ),
  square(
    size: 50pt,
    fill: gradient.radial(..color.map.rainbow, space: color.hsl, center: (0%, 100%)),
  ),
  square(
    size: 50pt,
    fill: gradient.radial(..color.map.rainbow, space: color.hsl, center: (100%, 0%)),
  ),
  square(
    size: 50pt,
    fill: gradient.radial(..color.map.rainbow, space: color.hsl, center: (100%, 100%)),
  ),
)

--- gradient-radial-radius ---
#square(
  size: 50pt,
  fill: gradient.radial(..color.map.rainbow, space: color.hsl, radius: 10%),
)
#square(
  size: 50pt,
  fill: gradient.radial(..color.map.rainbow, space: color.hsl, radius: 72%),
)

--- gradient-radial-focal-center-and-radius ---
#circle(
  radius: 25pt,
  fill: gradient.radial(white, rgb("#8fbc8f"), focal-center: (35%, 35%), focal-radius: 5%),
)
#circle(
  radius: 25pt,
  fill: gradient.radial(white, rgb("#8fbc8f"), focal-center: (75%, 35%), focal-radius: 5%),
)

--- gradient-radial-relative-parent ---
// The image should look as if there is a single gradient that is being used for
// both the page and the rectangles.
#let grad = gradient.radial(red, blue, green, purple, relative: "parent");
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 50pt,
  width: 50pt,
  margin: 2.5pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#place(top + right, my-rect)
#place(bottom + center, rotate(45deg, my-rect))

--- gradient-radial-relative-self ---
// The image should look as if there are multiple gradients, one for each
// rectangle.
#let grad = gradient.radial(red, blue, green, purple, relative: "self");
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 50pt,
  width: 50pt,
  margin: 2.5pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#place(top + right, my-rect)
#place(bottom + center, rotate(45deg, my-rect))

--- gradient-radial-text ---
// Test that gradient fills on text.
// The solid bar gradients are used to make sure that all transforms are
// correct: if you can see the text through the bar, then the gradient is
// misaligned to its reference container.
#set page(width: 200pt, height: auto, margin: 10pt)
#set par(justify: true)
#set text(fill: gradient.radial(red, blue))
#lorem(30)

--- gradient-conic ---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsv),
)

--- gradient-conic-center-shifted-1 ---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsv, center: (10%, 10%)),
)

--- gradient-conic-center-shifted-2 ---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsv, center: (90%, 90%)),
)

--- gradient-conic-angled ---
#square(
  size: 50pt,
  fill: gradient.conic(..color.map.rainbow, space: color.hsv, angle: 90deg),
)

--- gradient-conic-oklab ---
// Test in Oklab space for reference.
#set page(
  width: 100pt,
  height: 100pt,
  fill: gradient.conic(red, purple, space: oklab)
)

--- gradient-conic-oklch ---
// Test in OkLCH space.
#set page(
  width: 100pt,
  height: 100pt,
  fill: gradient.conic(red, purple, space: oklch)
)

--- gradient-conic-hsv ---
// Test in HSV space.
#set page(
  width: 100pt,
  height: 100pt,
  fill: gradient.conic(red, purple, space: color.hsv)
)

--- gradient-conic-hsl ---
// Test in HSL space.
#set page(
  width: 100pt,
  height: 100pt,
  fill: gradient.conic(red, purple, space: color.hsl)
)

--- gradient-conic-relative-parent ---
// The image should look as if there is a single gradient that is being used for
// both the page and the rectangles.
#let grad = gradient.conic(red, blue, green, purple, relative: "parent");
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 50pt,
  width: 50pt,
  margin: 2.5pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#place(top + right, my-rect)
#place(bottom + center, rotate(45deg, my-rect))

--- gradient-conic-relative-self ---
// The image should look as if there are multiple gradients, one for each
// rectangle.
#let grad = gradient.conic(red, blue, green, purple, relative: "self");
#let my-rect = rect(width: 50%, height: 50%, fill: grad)
#set page(
  height: 50pt,
  width: 50pt,
  margin: 2.5pt,
  fill: grad,
  background: place(top + left, my-rect),
)
#place(top + right, my-rect)
#place(bottom + center, rotate(45deg, my-rect))

--- gradient-conic-stroke ---
#align(
  center + bottom,
  square(
    size: 50pt,
    fill: black,
    stroke: 10pt + gradient.conic(red, blue)
  )
)

--- gradient-conic-text ---
#set page(width: 200pt, height: auto, margin: 10pt)
#set par(justify: true)
#set text(fill: gradient.conic(red, blue, angle: 45deg))
#lorem(30)

--- gradient-text-bad-relative ---
// Make sure they don't work when `relative: "self"`.
// Hint: 17-61 make sure to set `relative: auto` on your text fill
// Error: 17-61 gradients and tilings on text must be relative to the parent
#set text(fill: gradient.linear(red, blue, relative: "self"))

--- gradient-text-global ---
// Test that gradient fills on text work for globally defined gradients.
#set page(width: 200pt, height: auto, margin: 10pt, background: {
  rect(width: 100%, height: 30pt, fill: gradient.linear(red, blue))
})
#set par(justify: true)
#set text(fill: gradient.linear(red, blue))
#lorem(30)

--- gradient-text-dir ---
// Sanity check that the direction works on text.
#set page(width: 200pt, height: auto, margin: 10pt, background: {
  rect(height: 100%, width: 30pt, fill: gradient.linear(dir: btt, red, blue))
})
#set par(justify: true)
#set text(fill: gradient.linear(dir: btt, red, blue))
#lorem(30)

--- gradient-text-in-container ---
// Test that gradient fills on text work for locally defined gradients.
#set page(width: auto, height: auto, margin: 10pt)
#show box: set text(fill: gradient.linear(..color.map.rainbow))
Hello, #box[World]!

--- gradient-text-rotate ---
// Test that gradients fills on text work with transforms.
#set page(width: auto, height: auto, margin: 10pt)
#show box: set text(fill: gradient.linear(..color.map.rainbow))
#rotate(45deg, box[World])

--- gradient-text-decoration ---
#set text(fill: gradient.linear(red, blue))

Hello #underline[World]! \
Hello #overline[World]! \
Hello #strike[World]! \

--- gradient-transformed ---
// Test whether gradients work well when they are contained within a transform.
#let grad = gradient.linear(red, blue, green, purple, relative: "parent");
#let my-rect = rect(width: 50pt, height: 50pt, fill: grad)
#set page(
  height: 50pt,
  width: 50pt,
  margin: 2.5pt,
)
#place(top + right, scale(x: 200%, y: 130%, my-rect))
#place(bottom + center, rotate(45deg, my-rect))
#place(horizon + center, scale(x: 200%, y: 130%, rotate(45deg, my-rect)))

--- gradient-presets ---
// Test all gradient presets.
#set page(width: 100pt, height: auto, margin: 0pt)
#set text(fill: white, size: 18pt)
#set text(top-edge: "bounds", bottom-edge: "bounds")

#let presets = (
  ("turbo", color.map.turbo),
  ("cividis", color.map.cividis),
  ("rainbow", color.map.rainbow),
  ("spectral", color.map.spectral),
  ("viridis", color.map.viridis),
  ("inferno", color.map.inferno),
  ("magma", color.map.magma),
  ("plasma", color.map.plasma),
  ("rocket", color.map.rocket),
  ("mako", color.map.mako),
  ("vlag", color.map.vlag),
  ("icefire", color.map.icefire),
  ("flare", color.map.flare),
  ("crest", color.map.crest),
)

#stack(
  spacing: 3pt,
  ..presets.map(((name, preset)) => block(
    width: 100%,
    height: 20pt,
    fill: gradient.linear(..preset),
    align(center + horizon, smallcaps(name)),
  ))
)

// Test that gradients are applied correctly on equations.

--- gradient-math-cancel ---
// Test on cancel
#show math.equation: set text(fill: gradient.linear(..color.map.rainbow))
#show math.equation: box

$ a dot cancel(5) = cancel(25) 5 x + cancel(5) 1 $

--- gradient-math-frac ---
// Test on frac
#show math.equation: set text(fill: gradient.linear(..color.map.rainbow))
#show math.equation: box

$ nabla dot bold(E) = frac(rho, epsilon_0) $

--- gradient-math-root ---
// Test on root
#show math.equation: set text(fill: gradient.linear(..color.map.rainbow))
#show math.equation: box

$ x_"1,2" = frac(-b plus.minus sqrt(b^2 - 4 a c), 2 a) $

--- gradient-math-mat ---
// Test on matrix
#show math.equation: set text(fill: gradient.linear(..color.map.rainbow))
#show math.equation: box

$ A = mat(
  1, 2, 3;
  4, 5, 6;
  7, 8, 9
) $

--- gradient-math-underover ---
// Test on underover
#show math.equation: set text(fill: gradient.linear(..color.map.rainbow))
#show math.equation: box

$ underline(X^2) $
$ overline("hello, world!") $

--- gradient-math-dir ---
// Test a different direction
#show math.equation: set text(fill: gradient.linear(..color.map.rainbow, dir: ttb))
#show math.equation: box

$ A = mat(
  1, 2, 3;
  4, 5, 6;
  7, 8, 9
) $

$ x_"1,2" = frac(-b plus.minus sqrt(b^2 - 4 a c), 2 a) $

--- gradient-math-misc ---
// Test miscellaneous
#show math.equation: set text(fill: gradient.linear(..color.map.rainbow))
#show math.equation: box

$ hat(x) = bar x bar = vec(x, y, z) = tilde(x) = dot(x) $
$ x prime = vec(1, 2, delim: "[") $
$ sum_(i in NN) 1 + i $
$ attach(
  Pi, t: alpha, b: beta,
  tl: 1, tr: 2+3, bl: 4+5, br: 6,
) $

--- gradient-math-radial ---
// Test radial gradient
#show math.equation: set text(fill: gradient.radial(..color.map.rainbow, center: (30%, 30%)))
#show math.equation: box

$ A = mat(
  1, 2, 3;
  4, 5, 6;
  7, 8, 9
) $

--- gradient-math-conic ---
// Test conic gradient
#show math.equation: set text(fill: gradient.conic(red, blue, angle: 45deg))
#show math.equation: box

$ A = mat(
  1, 2, 3;
  4, 5, 6;
  7, 8, 9
) $


--- gradient-kind ---
// Test gradient functions.
#test(gradient.linear(red, green, blue).kind(), gradient.linear)

--- gradient-stops ---
#test(gradient.linear(red, green, blue).stops(), ((red, 0%), (green, 50%), (blue, 100%)))

--- gradient-sample ---
#test(gradient.linear(red, green, blue, space: rgb).sample(0%), red)
#test(gradient.linear(red, green, blue, space: rgb).sample(25%), rgb("#97873b"))
#test(gradient.linear(red, green, blue, space: rgb).sample(50%), green)
#test(gradient.linear(red, green, blue, space: rgb).sample(75%), rgb("#17a08c"))
#test(gradient.linear(red, green, blue, space: rgb).sample(100%), blue)

--- gradient-space ---
#test(gradient.linear(red, green, space: rgb).space(), rgb)
#test(gradient.linear(red, green, space: oklab).space(), oklab)
#test(gradient.linear(red, green, space: oklch).space(), oklch)
#test(gradient.linear(red, green, space: cmyk).space(), cmyk)
#test(gradient.linear(red, green, space: luma).space(), luma)
#test(gradient.linear(red, green, space: color.linear-rgb).space(), color.linear-rgb)
#test(gradient.linear(red, green, space: color.hsl).space(), color.hsl)
#test(gradient.linear(red, green, space: color.hsv).space(), color.hsv)

--- gradient-relative ---
#test(gradient.linear(red, green, relative: "self").relative(), "self")
#test(gradient.linear(red, green, relative: "parent").relative(), "parent")
#test(gradient.linear(red, green).relative(), auto)

--- gradient-angle ---
#test(gradient.linear(red, green).angle(), 0deg)
#test(gradient.linear(red, green, dir: ltr).angle(), 0deg)
#test(gradient.linear(red, green, dir: rtl).angle(), 180deg)
#test(gradient.linear(red, green, dir: ttb).angle(), 90deg)
#test(gradient.linear(red, green, dir: btt).angle(), 270deg)

--- gradient-repeat ---
#test(
  gradient.linear(red, green, blue).repeat(2).stops(),
  ((red, 0%), (green, 25%), (blue, 50%), (red, 50%), (green, 75%), (blue, 100%))
)
#test(
  gradient.linear(red, green, blue).repeat(2, mirror: true).stops(),
  ((red, 0%), (green, 25%), (blue, 50%), (green, 75%), (red, 100%))
)

--- issue-2902-gradient-oklch-panic ---
// Minimal reproduction of #2902
#set page(width: 15cm, height: auto, margin: 1em)
#set block(width: 100%, height: 1cm, above: 2pt)

// Oklch
#block(fill: gradient.linear(red, purple, space: oklch))
#block(fill: gradient.linear(..color.map.rainbow, space: oklch))
#block(fill: gradient.linear(..color.map.plasma, space: oklch))

--- issue-2902-gradient-oklab-panic ---
#set page(width: 15cm, height: auto, margin: 1em)
#set block(width: 100%, height: 1cm, above: 2pt)

// Oklab
#block(fill: gradient.linear(red, purple, space: oklab))
#block(fill: gradient.linear(..color.map.rainbow, space: oklab))
#block(fill: gradient.linear(..color.map.plasma, space: oklab))

--- issue-gradient-cmyk-encode ---
// Test that CMYK works on gradients
#set page(margin: 0pt, width: 100pt, height: auto)

#let violet = cmyk(75%, 80%, 0%, 0%)
#let blue = cmyk(75%, 30%, 0%, 0%)

#rect(
  width: 100%,
  height: 10pt,
  fill: gradient.linear(violet, blue)
)

#rect(
  width: 100%,
  height: 10pt,
  fill: gradient.linear(rgb(violet), rgb(blue))
)

// In PDF format, this gradient can look different from the others.
// This is because PDF readers do weird things with CMYK.
#rect(
  width: 100%,
  height: 10pt,
  fill: gradient.linear(violet, blue, space: cmyk)
)

--- issue-5819-gradient-repeat ---
// Ensure the gradient constructor generates monotonic stops which can be fed
// back into the gradient constructor itself.
#let my-gradient = gradient.linear(red, blue).repeat(5)
#let _ = gradient.linear(..my-gradient.stops())
#let my-gradient2 = gradient.linear(red, blue).repeat(5, mirror: true)
#let _ = gradient.linear(..my-gradient2.stops())

--- issue-6162-coincident-gradient-stops-export-png ---
// Ensure that multiple gradient stops with the same position
// don't cause a panic.
#rect(
  fill: gradient.linear(
    (red, 0%),
    (green, 0%),
    (blue, 100%),
  )
)
#rect(
  fill: gradient.linear(
    (red, 0%),
    (green, 100%),
    (blue, 100%),
  )
)
#rect(
  fill: gradient.linear(
    (white, 0%),
    (red, 50%),
    (green, 50%),
    (blue, 100%),
  )
)
