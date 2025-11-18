// SKIP

#let bg-rect(inner) = {
  rect(inset: 0pt, outset: 0pt, fill: luma(200), inner)
}

#let test-block(cap: "butt", dash: none, adjacent: none, radius: 0pt) = {
  let adjacent-stroke = if adjacent == none {
    none
  } else {
    (thickness: adjacent, paint: red, dash: dash)
  }
  bg-rect(
    block(
      height: 1.2cm,
      width: 1.5cm,
      stroke: (
        top: adjacent-stroke,
        right: (cap: cap, thickness: 6pt, dash: dash, paint: green.transparentize(40%)),
        bottom: (cap: cap, thickness: 2pt, dash: dash, paint: blue.transparentize(40%)),
        left: adjacent-stroke
      ),
      radius: radius,
    )
  )
}

#let another-block(cap: "butt", radius: 6pt, adjacent: none) = {
  let adjacent-stroke = if adjacent != none {
    adjacent
  } else {
    none
  }
  bg-rect(
    block(
      height: 1.2cm,
      width: 1.5cm,
      stroke: (
        top: adjacent-stroke,
        right: (cap: cap, thickness: 6pt),
        bottom: (cap: cap, thickness: 6pt),
        left: adjacent-stroke,
      ),
      radius: radius,
    )
  )
}
