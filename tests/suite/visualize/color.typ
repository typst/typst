// Test color modification methods.

--- color-mix ---
// Compare both ways.
#test-repr(rgb(0%, 30.2%, 70.2%), rgb("004db3"))

// Alpha channel.
#test(rgb(255, 0, 0, 50%), rgb("ff000080"))

// Test color modification methods.
#test(rgb(25, 35, 45).lighten(10%), rgb(48, 57, 66))
#test(rgb(40, 30, 20).darken(10%), rgb(36, 27, 18))
#test(rgb("#133337").negate(space: rgb), rgb(236, 204, 200))
#test(white.lighten(100%), white)

// Color mixing, in Oklab space by default.
#test(rgb(color.mix(rgb("#ff0000"), rgb("#00ff00"))), rgb("#d0a800"))
#test(rgb(color.mix(rgb("#ff0000"), rgb("#00ff00"), space: oklab)), rgb("#d0a800"))
#test(rgb(color.mix(rgb("#ff0000"), rgb("#00ff00"), space: rgb)), rgb("#808000"))

#test(rgb(color.mix(red, green, blue)), rgb("#909282"))
#test(rgb(color.mix(red, blue, green)), rgb("#909282"))
#test(rgb(color.mix(blue, red, green)), rgb("#909282"))

// Mix with weights.
#test(rgb(color.mix((red, 50%), (green, 50%))), rgb("#c0983b"))
#test(rgb(color.mix((red, 0.5), (green, 0.5))), rgb("#c0983b"))
#test(rgb(color.mix((red, 5), (green, 5))), rgb("#c0983b"))
#test(rgb(color.mix((green, 5), (white, 0), (red, 5))), rgb("#c0983b"))
#test(color.mix((rgb("#aaff00"), 25%), (rgb("#aa00ff"), 75%), space: rgb), rgb("#aa40bf"))
#test(color.mix((rgb("#aaff00"), 50%), (rgb("#aa00ff"), 50%), space: rgb), rgb("#aa8080"))
#test(color.mix((rgb("#aaff00"), 75%), (rgb("#aa00ff"), 25%), space: rgb), rgb("#aabf40"))

// Mix in hue-based space.
#test(rgb(color.mix(red, blue, space: color.hsl)), rgb("#c408ff"))
#test(rgb(color.mix((red, 50%), (blue, 100%), space: color.hsl)), rgb("#5100f8"))
// Error: 6-51 cannot mix more than two colors in a hue-based space
#rgb(color.mix(red, blue, white, space: color.hsl))

--- color-conversion ---
// Test color conversion method kinds
#test(rgb(rgb(10, 20, 30)).space(), rgb)
#test(color.linear-rgb(rgb(10, 20, 30)).space(), color.linear-rgb)
#test(oklab(rgb(10, 20, 30)).space(), oklab)
#test(oklch(rgb(10, 20, 30)).space(), oklch)
#test(color.hsl(rgb(10, 20, 30)).space(), color.hsl)
#test(color.hsv(rgb(10, 20, 30)).space(), color.hsv)
#test(cmyk(rgb(10, 20, 30)).space(), cmyk)
#test(luma(rgb(10, 20, 30)).space(), luma)

#test(rgb(color.linear-rgb(10, 20, 30)).space(), rgb)
#test(color.linear-rgb(color.linear-rgb(10, 20, 30)).space(), color.linear-rgb)
#test(oklab(color.linear-rgb(10, 20, 30)).space(), oklab)
#test(oklch(color.linear-rgb(10, 20, 30)).space(), oklch)
#test(color.hsl(color.linear-rgb(10, 20, 30)).space(), color.hsl)
#test(color.hsv(color.linear-rgb(10, 20, 30)).space(), color.hsv)
#test(cmyk(color.linear-rgb(10, 20, 30)).space(), cmyk)
#test(luma(color.linear-rgb(10, 20, 30)).space(), luma)

#test(rgb(oklab(10%, 20%, 30%)).space(), rgb)
#test(color.linear-rgb(oklab(10%, 20%, 30%)).space(), color.linear-rgb)
#test(oklab(oklab(10%, 20%, 30%)).space(), oklab)
#test(oklch(oklab(10%, 20%, 30%)).space(), oklch)
#test(color.hsl(oklab(10%, 20%, 30%)).space(), color.hsl)
#test(color.hsv(oklab(10%, 20%, 30%)).space(), color.hsv)
#test(cmyk(oklab(10%, 20%, 30%)).space(), cmyk)
#test(luma(oklab(10%, 20%, 30%)).space(), luma)

#test(rgb(oklch(60%, 40%, 0deg)).space(), rgb)
#test(color.linear-rgb(oklch(60%, 40%, 0deg)).space(), color.linear-rgb)
#test(oklab(oklch(60%, 40%, 0deg)).space(), oklab)
#test(oklch(oklch(60%, 40%, 0deg)).space(), oklch)
#test(color.hsl(oklch(60%, 40%, 0deg)).space(), color.hsl)
#test(color.hsv(oklch(60%, 40%, 0deg)).space(), color.hsv)
#test(cmyk(oklch(60%, 40%, 0deg)).space(), cmyk)
#test(luma(oklch(60%, 40%, 0deg)).space(), luma)

#test(rgb(color.hsl(10deg, 20%, 30%)).space(), rgb)
#test(color.linear-rgb(color.hsl(10deg, 20%, 30%)).space(), color.linear-rgb)
#test(oklab(color.hsl(10deg, 20%, 30%)).space(), oklab)
#test(oklch(color.hsl(10deg, 20%, 30%)).space(), oklch)
#test(color.hsl(color.hsl(10deg, 20%, 30%)).space(), color.hsl)
#test(color.hsv(color.hsl(10deg, 20%, 30%)).space(), color.hsv)
#test(cmyk(color.hsl(10deg, 20%, 30%)).space(), cmyk)
#test(luma(color.hsl(10deg, 20%, 30%)).space(), luma)

#test(rgb(color.hsv(10deg, 20%, 30%)).space(), rgb)
#test(color.linear-rgb(color.hsv(10deg, 20%, 30%)).space(), color.linear-rgb)
#test(oklab(color.hsv(10deg, 20%, 30%)).space(), oklab)
#test(oklch(color.hsv(10deg, 20%, 30%)).space(), oklch)
#test(color.hsl(color.hsv(10deg, 20%, 30%)).space(), color.hsl)
#test(color.hsv(color.hsv(10deg, 20%, 30%)).space(), color.hsv)
#test(cmyk(color.hsv(10deg, 20%, 30%)).space(), cmyk)
#test(luma(color.hsv(10deg, 20%, 30%)).space(), luma)

#test(rgb(cmyk(10%, 20%, 30%, 40%)).space(), rgb)
#test(color.linear-rgb(cmyk(10%, 20%, 30%, 40%)).space(), color.linear-rgb)
#test(oklab(cmyk(10%, 20%, 30%, 40%)).space(), oklab)
#test(oklch(cmyk(10%, 20%, 30%, 40%)).space(), oklch)
#test(color.hsl(cmyk(10%, 20%, 30%, 40%)).space(), color.hsl)
#test(color.hsv(cmyk(10%, 20%, 30%, 40%)).space(), color.hsv)
#test(cmyk(cmyk(10%, 20%, 30%, 40%)).space(), cmyk)
#test(luma(cmyk(10%, 20%, 30%, 40%)).space(), luma)

#test(rgb(luma(10%)).space(), rgb)
#test(color.linear-rgb(luma(10%)).space(), color.linear-rgb)
#test(oklab(luma(10%)).space(), oklab)
#test(oklch(luma(10%)).space(), oklch)
#test(color.hsl(luma(10%)).space(), color.hsl)
#test(color.hsv(luma(10%)).space(), color.hsv)
#test(cmyk(luma(10%)).space(), cmyk)
#test(luma(luma(10%)).space(), luma)

#test(rgb(1, 2, 3).to-hex(), "#010203")
#test(rgb(1, 2, 3, 4).to-hex(), "#01020304")
#test(luma(40).to-hex(), "#282828")
#test-repr(cmyk(4%, 5%, 6%, 7%).to-hex(), "#e0dcda")
#test-repr(rgb(cmyk(4%, 5%, 6%, 7%)), rgb(87.84%, 86.27%, 85.49%, 100%))
#test-repr(rgb(luma(40%)), rgb(40%, 40%, 40%))
#test-repr(cmyk(luma(40)), cmyk(63.24%, 57.33%, 56.49%, 75.88%))
#test-repr(cmyk(rgb(1, 2, 3)), cmyk(66.67%, 33.33%, 0%, 98.82%))
#test-repr(luma(rgb(1, 2, 3)), luma(0.73%))
#test-repr(color.hsl(luma(40)), color.hsl(0deg, 0%, 15.69%))
#test-repr(color.hsv(luma(40)), color.hsv(0deg, 0%, 15.69%))
#test-repr(color.linear-rgb(luma(40)), color.linear-rgb(2.12%, 2.12%, 2.12%))
#test-repr(color.linear-rgb(rgb(1, 2, 3)), color.linear-rgb(0.03%, 0.06%, 0.09%))
#test-repr(color.hsl(rgb(1, 2, 3)), color.hsl(-150deg, 50%, 0.78%))
#test-repr(color.hsv(rgb(1, 2, 3)), color.hsv(-150deg, 66.67%, 1.18%))
#test-repr(oklab(luma(40)), oklab(27.68%, 0.0, 0.0, 100%))
#test-repr(oklab(rgb(1, 2, 3)), oklab(8.23%, -0.004, -0.007, 100%))
#test-repr(oklch(oklab(40%, 0.2, 0.2)), oklch(40%, 0.283, 45deg, 100%))
#test-repr(oklch(luma(40)), oklch(27.68%, 0.0, 72.49deg, 100%))
#test-repr(oklch(rgb(1, 2, 3)), oklch(8.23%, 0.008, 240.75deg, 100%))

--- color-spaces ---
// The different color spaces
#let col = rgb(50%, 64%, 16%)
#box(square(size: 9pt, fill: col))
#box(square(size: 9pt, fill: rgb(col)))
#box(square(size: 9pt, fill: oklab(col)))
#box(square(size: 9pt, fill: oklch(col)))
#box(square(size: 9pt, fill: luma(col)))
#box(square(size: 9pt, fill: cmyk(col)))
#box(square(size: 9pt, fill: color.linear-rgb(col)))
#box(square(size: 9pt, fill: color.hsl(col)))
#box(square(size: 9pt, fill: color.hsv(col)))

--- color-space ---
// Test color kind method.
#test(rgb(1, 2, 3, 4).space(), rgb)
#test(cmyk(4%, 5%, 6%, 7%).space(), cmyk)
#test(luma(40).space(), luma)
#test(rgb(1, 2, 3, 4).space() != luma, true)

--- color-components ---
// Test color '.components()' without conversions

#let test-components(col, ref, has-alpha: true) = {
  // Perform an approximate scalar comparison.
  let are-equal((a, b)) = {
    let to-float(x) = if type(x) == angle { x.rad() } else { float(x) }
    let epsilon = 1e-4 // The maximum error between both numbers
    test(type(a), type(b))
    calc.abs(to-float(a) - to-float(b)) < epsilon
  }

  let ref-without-alpha = if has-alpha { ref.slice(0, -1) } else { ref }
  test(col.components().len(), ref.len())
  assert(col.components().zip(ref).all(are-equal))
  assert(col.components(alpha: false).zip(ref-without-alpha).all(are-equal))
}
#test-components(rgb(1, 2, 3, 4), (0.39%, 0.78%, 1.18%, 1.57%))
#test-components(luma(40), (15.69%, 100%))
#test-components(luma(40, 50%), (15.69%, 50%))
#test-components(cmyk(4%, 5%, 6%, 7%), (4%, 5%, 6%, 7%), has-alpha: false)
#test-components(oklab(10%, 0.2, 0.4), (10%, 0.2, 0.4, 100%))
#test-components(oklch(10%, 0.2, 90deg), (10%, 0.2, 90deg, 100%))
#test-components(oklab(10%, 50%, 200%), (10%, 0.2, 0.8, 100%))
#test-components(oklch(10%, 50%, 90deg), (10%, 0.2, 90deg, 100%))
#test-components(color.linear-rgb(10%, 20%, 30%), (10%, 20%, 30%, 100%))
#test-components(color.hsv(10deg, 20%, 30%), (10deg, 20%, 30%, 100%))
#test-components(color.hsl(10deg, 20%, 30%), (10deg, 20%, 30%, 100%))

--- color-luma ---
// Test gray color conversion.
#stack(dir: ltr, rect(fill: luma(0)), rect(fill: luma(80%)))

--- color-rgb-out-of-range ---
// Error for values that are out of range.
// Error: 11-14 number must be between 0 and 255
#test(rgb(-30, 15, 50))

--- color-rgb-bad-string ---
// Error: 6-11 color string contains non-hexadecimal letters
#rgb("lol")

--- color-rgb-missing-argument-red ---
// Error: 2-7 missing argument: red component
#rgb()

--- color-rgb-missing-argument-blue ---
// Error: 2-11 missing argument: blue component
#rgb(0, 1)

--- color-rgb-bad-type ---
// Error: 21-26 expected integer or ratio, found boolean
#rgb(10%, 20%, 30%, false)

--- color-luma-unexpected-argument ---
// Error: 10-20 unexpected argument: key
#luma(1, key: "val")

--- color-mix-bad-amount-type ---
// Error: 12-24 expected float or ratio, found string
// Error: 26-39 expected float or ratio, found string
#color.mix((red, "yes"), (green, "no"), (green, 10%))

--- color-mix-bad-value ---
// Error: 12-23 expected a color or color-weight pair
#color.mix((red, 1, 2))

--- color-mix-bad-space-type ---
// Error: 31-38 expected `rgb`, `luma`, `cmyk`, `oklab`, `oklch`, `color.linear-rgb`, `color.hsl`, or `color.hsv`, found string
#color.mix(red, green, space: "cyber")

--- color-mix-bad-space-value-1 ---
// Error: 31-36 expected `rgb`, `luma`, `cmyk`, `oklab`, `oklch`, `color.linear-rgb`, `color.hsl`, or `color.hsv`
#color.mix(red, green, space: image)

--- color-mix-bad-space-value-2 ---
// Error: 31-41 expected `rgb`, `luma`, `cmyk`, `oklab`, `oklch`, `color.linear-rgb`, `color.hsl`, or `color.hsv`
#color.mix(red, green, space: calc.round)

--- color-cmyk-ops ---
// Test CMYK color conversion.
#let c = cmyk(50%, 64%, 16%, 17%)
#stack(
  dir: ltr,
  spacing: 1fr,
  rect(width: 1cm, fill: cmyk(69%, 11%, 69%, 41%)),
  rect(width: 1cm, fill: c),
  rect(width: 1cm, fill: c.negate(space: cmyk)),
)

#for x in range(0, 11) {
  box(square(size: 9pt, fill: c.lighten(x * 10%)))
}
#for x in range(0, 11) {
  box(square(size: 9pt, fill: c.darken(x * 10%)))
}

--- color-outside-srgb-gamut ---
// Colors outside the sRGB gamut.
#box(square(size: 9pt, fill: oklab(90%, -0.2, -0.1)))
#box(square(size: 9pt, fill: oklch(50%, 0.5, 0deg)))

--- color-rotate-hue ---
// Test hue rotation
#let col = rgb(50%, 64%, 16%)

// Oklch
#for x in range(0, 11) {
  box(square(size: 9pt, fill: rgb(col).rotate(x * 36deg)))
}

// HSL
#for x in range(0, 11) {
  box(square(size: 9pt, fill: rgb(col).rotate(x * 36deg, space: color.hsl)))
}

// HSV
#for x in range(0, 11) {
  box(square(size: 9pt, fill: rgb(col).rotate(x * 36deg, space: color.hsv)))
}

--- color-saturation ---
// Test saturation
#let col = color.hsl(180deg, 0%, 50%)
#for x in range(0, 11) {
  box(square(size: 9pt, fill: col.saturate(x * 10%)))
}

#let col = color.hsl(180deg, 100%, 50%)
#for x in range(0, 11) {
  box(square(size: 9pt, fill: col.desaturate(x * 10%)))
}

#let col = color.hsv(180deg, 0%, 50%)
#for x in range(0, 11) {
  box(square(size: 9pt, fill: col.saturate(x * 10%)))
}

#let col = color.hsv(180deg, 100%, 50%)
#for x in range(0, 11) {
  box(square(size: 9pt, fill: col.desaturate(x * 10%)))
}

--- color-luma-ops ---
// Test gray color modification.
#test-repr(luma(20%).lighten(50%), luma(60%))
#test-repr(luma(80%).darken(20%), luma(64%))
#test-repr(luma(80%).negate(space: luma), luma(20%))

--- color-transparentize ---
// Test alpha modification.
#test-repr(luma(100%, 100%).transparentize(50%), luma(100%, 50%))
#test-repr(luma(100%, 100%).transparentize(75%), luma(100%, 25%))
#test-repr(luma(100%, 50%).transparentize(50%), luma(100%, 25%))
#test-repr(luma(100%, 10%).transparentize(250%), luma(100%, 0%))
#test-repr(luma(100%, 40%).transparentize(-50%), luma(100%, 70%))
#test-repr(luma(100%, 0%).transparentize(-100%), luma(100%, 100%))

--- color-opacify ---
#test-repr(luma(100%, 50%).opacify(50%), luma(100%, 75%))
#test-repr(luma(100%, 20%).opacify(100%), luma(100%, 100%))
#test-repr(luma(100%, 100%).opacify(250%), luma(100%, 100%))
#test-repr(luma(100%, 50%).opacify(-50%), luma(100%, 25%))
#test-repr(luma(100%, 0%).opacify(0%), luma(100%, 0%))

--- issue-color-mix-luma ---
// When mixing luma colors, we accidentally used the wrong component.
#rect(fill: gradient.linear(black, silver, space: luma))

--- issue-4361-transparency-leak ---
// Ensure that transparency doesn't leak from shapes to images in PDF. The PNG
// test doesn't validate it, but at least we can discover regressions on the PDF
// output with a PDF comparison script.
#rect(fill: red.transparentize(50%))
#image("/assets/images/tiger.jpg", width: 45pt)
