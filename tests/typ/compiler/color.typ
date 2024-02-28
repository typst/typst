// Test color modification methods.

---
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

---
// The the different color spaces
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

---
// Colors outside the sRGB gamut.
#box(square(size: 9pt, fill: oklab(90%, -0.2, -0.1)))
#box(square(size: 9pt, fill: oklch(50%, 0.5, 0deg)))

---
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

---
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

---
// Test gray color modification.
// Ref: false
#test-repr(luma(20%).lighten(50%), luma(60%))
#test-repr(luma(80%).darken(20%), luma(64%))
#test-repr(luma(80%).negate(space: luma), luma(20%))

---
// Test alpha modification.
// Ref: false
#test-repr(luma(100%, 100%).transparentize(50%), luma(100%, 50%))
#test-repr(luma(100%, 100%).transparentize(75%), luma(100%, 25%))
#test-repr(luma(100%, 50%).transparentize(50%), luma(100%, 25%))
#test-repr(luma(100%, 10%).transparentize(250%), luma(100%, 0%))
#test-repr(luma(100%, 40%).transparentize(-50%), luma(100%, 70%))
#test-repr(luma(100%, 0%).transparentize(-100%), luma(100%, 100%))

#test-repr(luma(100%, 50%).opacify(50%), luma(100%, 75%))
#test-repr(luma(100%, 20%).opacify(100%), luma(100%, 100%))
#test-repr(luma(100%, 100%).opacify(250%), luma(100%, 100%))
#test-repr(luma(100%, 50%).opacify(-50%), luma(100%, 25%))
#test-repr(luma(100%, 0%).opacify(0%), luma(100%, 0%))
