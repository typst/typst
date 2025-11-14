--- grid-headers paged pdftags pdfstandard(ua-1) ---
#set page(width: auto, height: 12em)
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..range(0, 6).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

--- grid-headers-no-repeat paged ---
// Disable repetition
#set page(width: auto, height: 12em)
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow),
    repeat: false
  ),
  ..range(0, 6).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

--- grid-headers-gutter paged ---
#set page(width: auto, height: 12em)
#table(
  columns: 5,
  align: center + horizon,
  gutter: 3pt,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow),
  ),
  ..range(0, 6).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

--- grid-header-relative-row-sizes paged ---
// Relative lengths
#set page(height: 10em)
#table(
  rows: (30%, 30%, auto),
  table.header(
    [*A*],
    [*B*]
  ),
  [C],
  [C]
)

--- grid-header-cell-with-y paged ---
#grid(
  grid.cell(y: 1)[a],
  grid.header(grid.cell(y: 0)[b]),
  grid.cell(y: 2)[c]
)

--- grid-header-cell-with-x paged ---
#grid(
  columns: 2,
  stroke: black,
  inset: 5pt,
  grid.header(grid.cell(x: 0)[b1], grid.cell(x: 0)[b2]),
  // This should skip the header
  grid.cell(x: 1)[c]
)

--- grid-header-last-child paged ---
// When the header is the last grid child, it shouldn't include the gutter row
// after it, because there is none.
#grid(
  columns: 2,
  gutter: 3pt,
  grid.header(
    [a], [b],
    [c], [d]
  )
)

--- grid-header-nested paged ---
#set page(height: 14em)
#let t(n) = table(
  columns: 3,
  align: center + horizon,
  gutter: 3pt,
  table.header(
    table.cell(colspan: 3)[*Cool Zone #n*],
    [*Name*], [*Num*], [*Data*]
  ),
  ..range(0, 5).map(i => ([\##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456])).flatten()
)
#grid(
  gutter: 3pt,
  t(0),
  t(1)
)

--- grid-header-hline-and-vline paged ---
// Test line positioning in header
#table(
  columns: 3,
  stroke: none,
  table.hline(stroke: red, end: 2),
  table.vline(stroke: red, end: 3),
  table.header(
    table.hline(stroke: aqua, start: 2),
    table.vline(stroke: aqua, start: 3), [*A*], table.hline(stroke: orange), table.vline(stroke: orange), [*B*],
    [*C*], [*D*]
  ),
  [a], [b],
  [c], [d],
  [e], [f]
)

--- grid-header-not-at-first-row paged ---
#grid(
  [a],
  grid.header([b])
)

--- grid-header-not-at-first-row-two-columns paged ---
#grid(
  columns: 2,
  [a],
  grid.header([b])
)

--- grid-header-multiple paged ---
#grid(
  grid.header([a]),
  grid.header([b]),
  [a],
)

--- grid-header-multiple-unordered paged ---
#set page(height: 4em)
#grid(
  grid.header(grid.cell(x: 0, y: 4)[y]),
  grid.header([x]),
  [a],
  [b],
  [c],
  [d],
  [e],
  [f],
)

--- grid-header-skip paged ---
#grid(
  columns: 2,
  [x], [y],
  grid.header([a]),
  grid.header([b]),
  grid.cell(x: 1)[c], [d],
  grid.header([e]),
  [f], grid.cell(x: 1)[g]
)

--- grid-header-skip-unordered paged ---
#grid(
  columns: 2,
  [a],
  grid.header(grid.cell(x: 0, y: 2)[y]),
  [b],
  grid.header([x]),
  [c]
)

--- grid-header-rowbreak-auto-pos paged ---
#grid(
  columns: 2,
  [x],
  grid.hline(stroke: red),
  grid.header([a]),
  grid.hline(stroke: 3pt),
  [y],
  grid.header(),
  [z],
)

--- grid-header-rowbreak-fixed-pos paged ---
#grid(
  columns: 2,
  [z],
  grid.hline(stroke: red),
  grid.header(grid.cell(x: 0)[b]),
  grid.hline(stroke: 3pt),
  [w],
  [j],
  grid.header(grid.cell(x: 0, y: 9)[c]),
  [k]
)

--- grid-header-rowbreak-mixed-pos paged ---
#grid(
  columns: 2,
  [a],
  grid.header([x], grid.cell(x: 0)[b]),
  [c],
  grid.hline(stroke: red),
  grid.header([y], grid.cell(x: 0, y: 8)[d]),
  grid.hline(stroke: 3pt),
  [e]
)

--- grid-header-rowbreak-auto-and-fixed-pos paged ---
#grid(
  columns: 2,
  [a],
  grid.header([x]),
  [b],
  grid.header(grid.cell(x: 0, y: 3)[y]),
  [c]
)

--- grid-header-too-large-non-repeating-orphan paged ---
#set page(height: 8em)
#grid(
  grid.header(
    [a\ ] * 5,
    repeat: false,
  ),
  [b]
)

--- grid-header-too-large-repeating-orphan paged ---
#set page(height: 8em)
#grid(
  grid.header(
    [a\ ] * 5,
    repeat: true,
  ),
  [b]
)

--- grid-header-too-large-repeating-orphan-with-footer paged ---
#set page(height: 8em)
#grid(
  grid.header(
    [a\ ] * 5,
    repeat: true,
  ),
  [b],
  grid.footer(
    [c],
    repeat: true,
  )
)

--- grid-header-too-large-repeating-orphan-not-at-first-row paged ---
#set page(height: 8em)
#grid(
  [b],
  grid.header(
    [a\ ] * 5,
    repeat: true,
  ),
  [c],
)

--- table-header-in-grid paged ---
// Error: 2:3-2:20 cannot use `table.header` as a grid header
// Hint: 2:3-2:20 use `grid.header` instead
#grid(
  table.header([a]),
  [a],
)

--- grid-header-in-table paged ---
// Error: 2:3-2:19 cannot use `grid.header` as a table header
// Hint: 2:3-2:19 use `table.header` instead
#table(
  grid.header([a]),
  [a],
)

--- grid-header-in-grid-header paged ---
// Error: 14-28 cannot place a grid header within another header or footer
#grid.header(grid.header[a])

--- table-header-in-grid-header paged ---
// Error: 14-29 cannot place a table header within another header or footer
#grid.header(table.header[a])

--- grid-header-in-table-header paged ---
// Error: 15-29 cannot place a grid header within another header or footer
#table.header(grid.header[a])

--- table-header-in-table-header paged ---
// Error: 15-30 cannot place a table header within another header or footer
#table.header(table.header[a])

--- grid-header-block-with-fixed-height paged ---
#set page(height: 15em)
#table(
  rows: (auto, 2.5em, auto),
  table.header(
    [*Hello*],
    [*World*]
  ),
  block(width: 2em, height: 20em, fill: red)
)

--- grid-header-and-rowspan-non-contiguous-1 paged ---
// Rowspan sizing algorithm doesn't do the best job at non-contiguous content
// ATM.
#set page(height: 15em)

#table(
  rows: (auto, 2.5em, 2em, auto, 5em),
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lines(15))
)

--- grid-header-and-rowspan-non-contiguous-2 paged ---
// Rowspan sizing algorithm doesn't do the best job at non-contiguous content
// ATM.
#set page(height: 15em)

#table(
  rows: (auto, 2.5em, 2em, auto, 5em),
  gutter: 3pt,
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lines(15))
)

--- grid-header-and-rowspan-non-contiguous-3 paged ---
// This should look right
#set page(height: 15em)

#table(
  rows: (auto, 2.5em, 2em, auto),
  gutter: 3pt,
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lines(15))
)

--- grid-header-and-rowspan-contiguous-1 paged ---
// Block should occupy all space
#set page(height: 15em)

#table(
  rows: (auto, 2.5em, 2em, auto),
  gutter: 3pt,
  inset: 0pt,
  table.header(
    [*H*],
    [*W*]
  ),
  table.cell(rowspan: 3, block(height: 2.5em + 2em + 20em, width: 100%, fill: red))
)

--- grid-header-and-rowspan-contiguous-2 paged ---
// Block should occupy all space
#set page(height: 15em)

#table(
  rows: (auto, 2.5em, 10em, 5em, auto),
  gutter: 3pt,
  inset: 0pt,
  table.header(
    [*H*],
    [*W*]
  ),
  table.cell(rowspan: 3, block(height: 2.5em + 2em + 20em, width: 100%, fill: red))
)

--- grid-header-and-large-auto-contiguous paged ---
// Block should occupy all space
#set page(height: 15em)

#table(
  rows: (auto, 4.5em, auto),
  gutter: 3pt,
  inset: 0pt,
  table.header(
    [*H*],
    [*W*]
  ),
  block(height: 2.5em + 2em + 20em, width: 100%, fill: red)
)

--- grid-header-lack-of-space paged ---
// Test lack of space for header + text.
#set page(height: 8em)

#table(
  rows: (auto, 2.5em, auto, auto, 10em),
  gutter: 3pt,
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lorem(80))
)

--- grid-header-orphan-prevention paged ---
// Orphan header prevention test
#set page(height: 12em)
#v(8em)
#grid(
  columns: 3,
  grid.header(
    [*Mui*], [*A*], grid.cell(rowspan: 2, fill: orange)[*B*],
    [*Header*], [*Header* #v(0.1em)]
  ),
  ..([Test], [Test], [Test]) * 20
)

--- grid-header-non-repeating-orphan-prevention paged ---
#set page(height: 5em)
#v(2em)
#grid(
  grid.header(repeat: false)[*Abc*],
  [a],
  [b],
  [c],
  [d]
)

--- grid-header-empty paged ---
// Empty header should just be a repeated blank row
#set page(height: 12em)
#table(
  columns: 4,
  align: center + horizon,
  table.header(),
  ..range(0, 4).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789])).flatten()
)

--- grid-header-containing-rowspan paged ---
// When a header has a rowspan with an empty row, it should be displayed
// properly
#set page(height: 10em)

#let count = counter("g")
#table(
  rows: (auto, 2em, auto, auto),
  table.header(
    [eeec],
    table.cell(rowspan: 2, count.step() + context count.display()),
  ),
  [d],
  block(width: 5em, fill: yellow, lorem(15)),
  [d]
)
#context count.display()

--- grid-header-no-expand-with-col-and-row-pos-cell paged ---
#set page(height: 10em)
#table(
  columns: 2,
  table.header(
    [a], [b],
    [c],
  ),
  // Error: 3-48 cell would conflict with header also spanning row 1
  // Hint: 3-48 try moving the cell or the header
  table.cell(x: 1, y: 1, rowspan: 2, lorem(80))
)

--- grid-header-no-expand-with-row-pos-cell paged ---
#set page(height: 10em)
#table(
  columns: 2,
  table.header(
    [a], [b],
    [c],
  ),
  // Error: 3-42 cell would conflict with header also spanning row 1
  // Hint: 3-42 try moving the cell or the header
  table.cell(y: 1, rowspan: 2, lorem(80))
)

--- grid-nested-with-headers paged ---
// Nested table with header should repeat both headers
#set page(height: 10em)
#table(
  table.header(
    [a]
  ),
  table(
    table.header(
      [b]
    ),
    [a\ b\ c\ d]
  )
)

--- grid-nested-headers paged ---
#set page(height: 12em)
#table(
  table.header(
    table(
      table.header(
        [b]
      ),
      [c],
      [d]
    )
  ),
  [a\ b]
)

--- grid-header-not-at-the-top paged ---
#set page(height: 5em)
#v(2em)
#grid(
  [a],
  [b],
  grid.header[*Abc*],
  [d],
  [e],
  [f],
)

--- grid-header-replace paged ---
#set page(height: 5em)
#v(1.5em)
#grid(
  grid.header[*Abc*],
  [a],
  [b],
  grid.header[*Def*],
  [d],
  [e],
  [f],
)

--- grid-header-replace-orphan paged ---
#set page(height: 5em)
#grid(
  grid.header[*Abc*],
  [a],
  [b],
  grid.header[*Def*],
  [d],
  [e],
  [f],
)

--- grid-header-replace-doesnt-fit paged ---
#set page(height: 5em)
#v(0.8em)
#grid(
  grid.header[*Abc*],
  [a],
  [b],
  grid.header[*Def*],
  [d],
  [e],
  [f],
)

--- grid-header-stroke-edge-cases paged ---
// Test header stroke priority edge case (last header row removed)
#set page(height: 8em)
#table(
  columns: 2,
  stroke: black,
  gutter: (auto, 3pt),
  table.header(
    [c], [d],
  ),
  ..(table.cell(stroke: aqua)[d],) * 8,
)

--- grid-header-hline-bottom paged ---
// Yellow line should be kept here
#set text(6pt)
#table(
  column-gutter: 3pt,
  inset: 1pt,
  table.header(
    [a],
    table.hline(stroke: yellow),
  ),
  table.cell(rowspan: 2)[b]
)

--- grid-header-hline-bottom-manually paged ---
// Red line should be kept here
#set page(height: 6em)
#set text(6pt)
#table(
  column-gutter: 3pt,
  inset: 1pt,
  table.header(
    table.hline(stroke: red, position: bottom),
    [a],
  ),
  [a],
  table.cell(stroke: aqua)[b]
)

--- grid-header-rowspan-base paged ---
#set page(height: 7em)
#set text(6pt)
#let full-block = block(width: 2em, height: 100%, fill: red)
#table(
  columns: 3,
  inset: 1.5pt,
  table.header(
    [a], full-block, table.cell(rowspan: 2, full-block),
    [b]
  )
)

--- grid-header-row-pos-cell-inside-conflicts-with-row-before paged ---
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 3,
  inset: 1.5pt,
  table.cell(y: 0)[a],
  table.header(
    table.hline(stroke: red),
    table.hline(y: 1, stroke: aqua),
    // Error: 5-24 cell would cause header to expand to non-empty row 0
    // Hint: 5-24 try moving its cells to available rows
    table.cell(y: 0)[b],
    [c]
  )
)

--- grid-header-row-pos-cell-inside-conflicts-with-row-before-after-first-empty-row paged ---
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 3,
  inset: 1.5pt,
  // Rows: Occupied, Empty, Occupied, Empty, Empty, ...
  // Should not be able to expand header from the second Empty to the second Occupied.
  table.cell(y: 0)[a],
  table.cell(y: 2)[a],
  table.header(
    table.hline(stroke: red),
    table.hline(y: 3, stroke: aqua),
    // Error: 5-24 cell would cause header to expand to non-empty row 2
    // Hint: 5-24 try moving its cells to available rows
    table.cell(y: 2)[b],
  )
)

--- grid-header-auto-pos-cell-inside-conflicts-with-row-after paged ---
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 2,
  inset: 1.5pt,
  table.cell(y: 1)[a],
  table.header(
    [b], [c],
    // Error: 6-7 cell would cause header to expand to non-empty row 1
    // Hint: 6-7 try moving its cells to available rows
    [d],
  ),
)

--- grid-header-row-pos-cell-inside-conflicts-with-row-after paged ---
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 2,
  inset: 1.5pt,
  table.cell(y: 2)[a],
  table.header(
    [b], [c],
    // Error: 5-24 cell would cause header to expand to non-empty row 2
    // Hint: 5-24 try moving its cells to available rows
    table.cell(y: 3)[d],
  ),
)

--- grid-header-collision-multiple-ordered paged ---
#grid(
  columns: 2,
  grid.cell(x: 0, y: 0)[a],
  grid.cell(x: 1, y: 0)[a],
  grid.cell(x: 0, y: 3)[a],
  grid.header(grid.cell(x: 0, y: 2)[y]),
  // Error: 15-39 cell would cause header to expand to non-empty row 3
  // Hint: 15-39 try moving its cells to available rows
  grid.header(grid.cell(x: 0, y: 3)[y]),
  grid.header(grid.cell(x: 0, y: 4)[y]),
)

--- grid-header-collision-multiple-unordered paged ---
#grid(
  columns: 2,
  grid.cell(x: 0, y: 0)[a],
  grid.cell(x: 1, y: 0)[a],
  grid.header(grid.cell(x: 0, y: 2)[y]),
  grid.header(grid.cell(x: 0, y: 3)[y]),
  grid.header(grid.cell(x: 0, y: 4)[y]),
  // Error: 3-27 cell would conflict with header also spanning row 3
  // Hint: 3-27 try moving the cell or the header
  grid.cell(x: 0, y: 3)[a]
)

--- grid-header-collision-multiple-rowspan paged ---
#grid(
  columns: 2,
  grid.cell(x: 0, y: 0)[a],
  grid.cell(x: 1, y: 0)[a],
  grid.header(grid.cell(x: 0, y: 2)[y]),
  grid.header(grid.cell(x: 0, y: 3)[y]),
  grid.header(grid.cell(x: 0, y: 4)[y]),
  // Error: 3-39 cell would conflict with header also spanning row 2
  // Hint: 3-39 try moving the cell or the header
  grid.cell(x: 0, y: 1, rowspan: 2)[a]
)

--- issue-5359-column-override-stays-inside-header paged ---
#table(
  columns: 3,
  [Outside],
  table.header(
    [A], table.cell(x: 1)[B], [C],
    table.cell(x: 1)[D],
  ),
)

--- issue-6666-auto-hlines-around-header paged ---
#table(
	columns: 2,
	table.hline(stroke: 2pt + blue),
	table.header([*foo*], [*bar*]),
	table.hline(stroke: 1.5pt + red),
	table.cell(colspan: 2)[_asdf_],
	table.hline(stroke: 1.5pt + red),
	[a], [b],
	[c], [d],
)
