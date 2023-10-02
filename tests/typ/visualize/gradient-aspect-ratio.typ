// Tests that the aspect ratio is taken into account to correct
// the angle of the gradient. All of the rectangles in each group
// should appear to have the same gradient angle.

---
#set page(height: 40pt, width: 50pt, margin: 0pt)

#let fill = gradient.linear(dir: 45deg, red, blue).sharp(2);
#stack(
  dir: ltr,
  rect(
    width: 10pt,
    height: 10pt,
    fill: fill,
  ),
  rect(
    width: 30pt,
    height: 10pt,
    fill: fill,
  ),
  rect(
    width: 10pt,
    height: 30pt,
    fill: fill,
  ),
)