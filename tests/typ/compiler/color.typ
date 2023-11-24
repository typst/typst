// Test color modification methods.

---
// Test CMYK color conversion.
#let c = cmyk(50%, 64%, 16%, 17%)
#stack(
  dir: ltr,
  spacing: 1fr,
  rect(width: 1cm, fill: cmyk(69%, 11%, 69%, 41%)),
  rect(width: 1cm, fill: c),
  rect(width: 1cm, fill: c.negate()),
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
/// Test alpha modification.
// Ref: false
#let orig = rgb(80%, 50%, 0%, 75%)
#let target = rgb(80%, 50%, 0%, 50%)
#test-repr(orig.with-alpha(50%), target)
#test-repr(orig.with-alpha(128), target)
#test-repr(oklab(orig).with-alpha(50%), oklab(target))
#test-repr(oklch(orig).with-alpha(50%), oklch(target))
#test-repr(color.linear-rgb(orig).with-alpha(50%), color.linear-rgb(target))
#test-repr(color.hsl(orig).with-alpha(50%), color.hsl(target))
#test-repr(color.hsv(orig).with-alpha(50%), color.hsv(target))

---
// Error: 2-26 cannot set alpha component of this color space
// Hint: 2-26 try converting your color to RGB first
#luma(50%).with-alpha(50)

---
// Test gray color modification.
// Ref: false
#test-repr(luma(20%).lighten(50%), luma(60%))
#test-repr(luma(80%).darken(20%), luma(64%))
#test-repr(luma(80%).negate(), luma(20%))
