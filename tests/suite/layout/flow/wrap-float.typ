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
#set page(width: 220pt, height: 260pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 50pt, height: 50pt, fill: aqua))
#place(top + left, float: true, wrap: true, dy: 80pt, clearance: 8pt,
  rect(width: 50pt, height: 50pt, fill: forest))
#lorem(90)

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
#columns(2)
#place(top + right, float: true, wrap: true, scope: "parent", clearance: 8pt,
  rect(width: 50pt, height: 60pt, fill: aqua))
#lorem(80)
