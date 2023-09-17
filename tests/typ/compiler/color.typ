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
#box(square(size: 9pt, fill: col.to-rgba()))
#box(square(size: 9pt, fill: col.to-oklab()))
#box(square(size: 9pt, fill: col.to-luma()))
#box(square(size: 9pt, fill: col.to-linear-rgb()))
#box(square(size: 9pt, fill: col.to-cmyk()))
#box(square(size: 9pt, fill: col.to-hsl()))
#box(square(size: 9pt, fill: col.to-hsv()))

---
// Test hue rotation

#let col = rgb(50%, 64%, 16%)

#for x in range(0, 11) {
  box(square(size: 9pt, fill: col.to-rgba().rotate(x * 36deg)))
}

#for x in range(0, 11) {
  box(square(size: 9pt, fill: col.to-hsv().rotate(x * 36deg)))
}

#for x in range(0, 11) {
  box(square(size: 9pt, fill: col.to-hsl().rotate(x * 36deg)))
}

---
// Test gray color modification.
// Ref: false
#test(repr(luma(20%).lighten(50%)), repr(luma(60%)))
#test(repr(luma(80%).darken(20%)), repr(luma(64%)))
#test(repr(luma(80%).negate()), repr(luma(20%)))
