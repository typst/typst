// Some splitting corner cases

---
// Inside the larger rowspan's range, there's an unbreakable rowspan and a
// breakable rowspan. This should work normally.
// The auto row will also expand ignoring the last fractional row.
#set page(height: 10em)
#table(
    gutter: 0.5em,
    columns: 2,
    rows: (2em,) * 10 + (auto, auto, 2em, 1fr),
    fill: (_, y) => if calc.even(y) { aqua } else { blue },
    table.cell(rowspan: 14, block(width: 2em, height: 2em * 10 + 2em + 5em, fill: red)[]),
    ..([a],) * 5,
    table.cell(rowspan: 3)[a\ b],
    table.cell(rowspan: 5, [a\ b\ c\ d\ e\ f\ g\ h]),
    [z]
)
