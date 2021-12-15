// Test placing a background image on a page.

---
#page(paper: "a10", flipped: true)
#font(fill: white)
#place(
  dx: -10pt,
  dy: -10pt,
  image(
    "../../res/tiger.jpg",
    fit: "cover",
    width: 100% + 20pt,
    height: 100% + 20pt,
  )
)
#align(bottom + right)[
  _Welcome to_ #underline[*Tigerland*]
]
