// Test vertical alignment with nested placement.

---
#box(
  fill: aqua,
  width: 30pt,
  height: 30pt,
  place(bottom,
    place(line(start: (0pt, 0pt), end: (20pt, 0pt), stroke: red + 3pt))
  )
)

---
#box(
  fill: aqua,
  width: 30pt,
  height: 30pt,
  {
    box(fill: yellow, {
      [Hello]
      place(horizon, line(start: (0pt, 0pt), end: (20pt, 0pt), stroke: red + 2pt))
    })
    place(horizon, line(start: (0pt, 0pt), end: (20pt, 0pt), stroke: green + 3pt))
  }
)

---
#box(fill: aqua)[
  #place(bottom + right)[Hi]
  Hello World \
  How are \
  you?
]

---
#box(fill: aqua)[
  #place(top + left, dx: 50%, dy: 50%)[Hi]
  #v(30pt)
  #line(length: 50pt)
]
