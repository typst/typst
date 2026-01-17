// Wrap-float demos and tests.

--- wrap-float-basic-right paged ---
#set page(width: 200pt, height: 200pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(60)

--- wrap-float-basic-left paged ---
#set page(width: 200pt, height: 200pt)
#place(top + left, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(60)

--- wrap-float-bottom-band paged ---
#set page(width: 200pt, height: 200pt)
#place(bottom + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(60)

--- wrap-float-multiple paged ---
#set page(width: 200pt, height: 220pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 45pt, height: 45pt, fill: aqua))
#place(top + left, float: true, wrap: true, dy: 60pt, clearance: 8pt,
  rect(width: 45pt, height: 45pt, fill: forest))
#lorem(60)

--- wrap-float-offset paged ---
#set page(width: 200pt, height: 200pt)
#place(top + right, float: true, wrap: true, dx: -6pt, dy: 6pt, clearance: 8pt,
  rect(width: 60pt, height: 70pt, fill: aqua))
#lorem(60)

--- wrap-float-mixed-font paged ---
#set page(width: 220pt, height: 220pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#text(size: 9pt)[
  #lorem(20)
]
#text(size: 14pt)[
  #lorem(20)
]
#text(size: 10pt)[
  #lorem(20)
]

--- wrap-float-columns-parent paged ---
#set page(width: 240pt, height: 240pt)
#columns(2)[
  #place(top + right, float: true, wrap: true, scope: "parent", clearance: 8pt,
    rect(width: 50pt, height: 60pt, fill: aqua))
  // Warning: 4-13 paragraph spans page break with changing wrap context; text may appear incorrectly indented on continuation page
  #lorem(80)
]

--- wrap-float-inline-position paged ---
// Wrap float appearing mid-page with text before and after.
#set page(width: 200pt, height: 300pt)
Before the float. #lorem(15)

#place(top + right, float: true, wrap: true, dy: 60pt, clearance: 8pt,
  rect(width: 50pt, height: 50pt, fill: aqua))
Text that wraps around the float which appears partway down the page.
#lorem(40)

--- wrap-float-two-same-side paged ---
// Two floats on same side, stacked vertically.
#set page(width: 200pt, height: 280pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 50pt, height: 40pt, fill: aqua))
#place(top + right, float: true, wrap: true, dy: 56pt, clearance: 8pt,
  rect(width: 50pt, height: 40pt, fill: forest))
#lorem(60)

--- wrap-float-different-heights paged ---
// Floats with different heights cause varying line widths.
#set page(width: 200pt, height: 260pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 50pt, height: 30pt, fill: aqua))
#place(top + left, float: true, wrap: true, dy: 50pt, clearance: 8pt,
  rect(width: 40pt, height: 60pt, fill: forest))
#lorem(60)

--- wrap-float-single-column paged ---
// Simple single column layout for baseline.
#set page(width: 180pt, height: 180pt)
#place(top + right, float: true, wrap: true, clearance: 6pt,
  rect(width: 50pt, height: 60pt, fill: aqua))
#lorem(40)
