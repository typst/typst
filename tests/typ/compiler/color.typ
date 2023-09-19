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
#box(square(size: 9pt, fill: luma(col)))
#box(square(size: 9pt, fill: cmyk(col)))
#box(square(size: 9pt, fill: color.linear-rgb(col)))
#box(square(size: 9pt, fill: color.hsl(col)))
#box(square(size: 9pt, fill: color.hsv(col)))

---
// Test hue rotation
#let col = rgb(50%, 64%, 16%)

#for x in range(0, 11) {
  box(square(size: 9pt, fill: rgb(col).rotate(x * 36deg)))
}

#for x in range(0, 11) {
  box(square(size: 9pt, fill: color.hsv(col).rotate(x * 36deg)))
}

#for x in range(0, 11) {
  box(square(size: 9pt, fill: color.hsl(col).rotate(x * 36deg)))
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
#test-repr(luma(80%).negate(), luma(20%))
