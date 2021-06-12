// Test grid cells that overflow to the next region.

---
#page(width: 5cm, height: 3cm)
#grid(
    columns: 2,
    gutter-rows: 3 * (8pt,),
    [Lorem ipsum dolor sit amet, consectetuer adipiscing elit.

    Aenean commodo ligula eget dolor. Aenean massa. Cum sociis natoque penatibus et magnis.],
    [Text that is rather short],
    [Fireflies],
    [Critical],
    [Decorum],
    [Rampage],
)

---
// Test a column that starts overflowing right after another row/column did
// that.
#page(width: 5cm, height: 2cm)
#grid(
    columns: 4 * (1fr,),
    gutter-rows: (10pt,),
    gutter-columns: (0pt, 10%),
    image("../../res/rhino.png"),
    align(right, rect(width: 100%, fill: eastern)[LoL]),
    "rofl",
    "\nA" * 3,
    "Ha!\n" * 3,
)

---
// Test two columns in the same row overflowing by a different amount.
#page(width: 5cm, height: 2cm)
#grid(
    columns: 3 * (1fr,),
    gutter-rows: (8pt,),
    gutter-columns: (0pt, 10%),
    [A], [B], [C],
    "Ha!\n" * 6,
    "rofl",
    "\nA" * 3,
    [hello],
    [darkness],
    [my old]
)

---
// Test grid within a grid, overflowing.
#page(width: 5cm, height: 2.25cm)
#grid(
    columns: 4 * (1fr,),
    gutter-rows: (10pt,),
    gutter-columns: (0pt, 10%),
    [A], [B], [C], [D],
    grid(columns: 2, [A], [B], "C\n"*3, [D]),
    align(right, rect(width: 100%, fill: eastern)[LoL]),
    "rofl",
    "E\n"*4,
)

---
// Test partition of `fr` units before and after multi-region layout.
#page(width: 5cm, height: 4cm)
#grid(
    columns: 2 * (1fr,),
    rows: (1fr, 2fr, auto, 1fr, 1cm),
    gutter-rows: 4 * (10pt,),
    rect(height: 100%, width: 100%, fill: #ff0000)[No height],
    [foo],
    rect(height: 100%, width: 100%, fill: #fc0030)[Still no height],
    [bar],
    [The nature of being itself is in question. Am I One? Am I Many? What is being alive?],
    [baz],
    [The answer],
    [42],
    [Other text of interest],
)
