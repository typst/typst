// Test grid cells that overflow to the next region.

---
#set page(width: 5cm, height: 3cm)
#grid(
  columns: 2,
  row-gutter: 8pt,
  [Lorem ipsum dolor sit amet.

  Aenean commodo ligula eget dolor. Aenean massa. Penatibus et magnis.],
  [Text that is rather short],
  [Fireflies],
  [Critical],
  [Decorum],
  [Rampage],
)

---
// Test a column that starts overflowing right after another row/column did
// that.
#set page(width: 5cm, height: 2cm)
#grid(
  columns: 4 * (1fr,),
  row-gutter: 10pt,
  column-gutter: (0pt, 10%),
  align(top, image("/files/rhino.png")),
  align(top, rect(inset: 0pt, fill: eastern, align(right)[LoL])),
  [rofl],
  [\ A] * 3,
  [Ha!\ ] * 3,
)

---
// Test two columns in the same row overflowing by a different amount.
#set page(width: 5cm, height: 2cm)
#grid(
  columns: 3 * (1fr,),
  row-gutter: 8pt,
  column-gutter: (0pt, 10%),
  [A], [B], [C],
  [Ha!\ ] * 6,
  [rofl],
  [\ A] * 3,
  [hello],
  [darkness],
  [my old]
)

---
// Test grid within a grid, overflowing.
#set page(width: 5cm, height: 2.25cm)
#grid(
  columns: 4 * (1fr,),
  row-gutter: 10pt,
  column-gutter: (0pt, 10%),
  [A], [B], [C], [D],
  grid(columns: 2, [A], [B], [C\ ]*3, [D]),
  align(top, rect(inset: 0pt, fill: eastern, align(right)[LoL])),
  [rofl],
  [E\ ]*4,
)
