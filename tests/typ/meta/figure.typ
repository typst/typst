// Test figures.

---
#set page(width: 150pt)
#set figure(numbering: "I")

We can clearly see that @fig-cylinder and
@tab-complex are relevant in this context.

#figure(
  table(columns: 2)[a][b],
  caption: [The basic table.],
) <tab-basic>

#figure(
  pad(y: -11pt, image("/cylinder.svg", height: 3cm)),
  caption: [The basic shapes.],
  numbering: "I",
) <fig-cylinder>

#figure(
  table(columns: 3)[a][b][c][d][e][f],
  caption: [The complex table.],
) <tab-complex>
