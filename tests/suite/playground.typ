--- playground html ---
// #block(
//   width: 2cm,
//   height: 1cm,
//   fill: rgb("239dad").lighten(50%),
//   inset: 5pt,
//   stroke: 2pt + blue,
// )[A]
// #html.div(style: "height: 20px")

#block(
  fill: gradient.radial(
    ..color.map.viridis,
    focal-center: (10%, 40%),
    focal-radius: 5%,
  ),
  width: 2cm,
  height: 2cm,
  stroke: 1pt,
  inset: 5pt,
)[I am a block]
