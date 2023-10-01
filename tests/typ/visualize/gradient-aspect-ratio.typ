// Tests that the aspect ratio is taken into account to correct
// the angle of the gradient.

---
#set page(height: 120pt, width: 110pt, margin: 0pt)
#set block(spacing: 0pt)

#for i in range(0, 360, step: 30) {
    stack(
        dir: ltr,
        rect(
            width: 10pt,
            height: 10pt,
            fill: gradient.linear(dir: i * 1deg, red, blue).sharp(2)
        ),
        rect(
            width: 100pt,
            height: 10pt,
            fill: gradient.linear(dir: i * 1deg, red, blue).sharp(2)
        )
    )
}