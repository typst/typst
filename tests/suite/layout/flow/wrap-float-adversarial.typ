// Adversarial wrap-float demos and tests.

--- wrap-float-footnote-bottom paged ---
#set page(width: 220pt, height: 220pt)
#set footnote(numbering: "1")
#place(bottom + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(30) #footnote[Footnote text that should stay with the reference.]
#lorem(30)

--- wrap-float-list-adjacent paged ---
#set page(width: 220pt, height: 220pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
+ First list item wraps around the float and continues.
+ Second item wraps as well and should align cleanly.

--- wrap-float-table-nearby paged ---
#set page(width: 240pt, height: 240pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(20)
#table(
  columns: 3,
  [A], [B], [C],
  [1], [2], [3],
  [4], [5], [6],
)
#lorem(20)

--- wrap-float-long-paragraph-guardrail paged ---
#set page(width: 260pt, height: 260pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 70pt, height: 90pt, fill: aqua))
#lorem(220)

--- wrap-float-overlap-zones paged ---
#set page(width: 240pt, height: 260pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 50pt, height: 70pt, fill: aqua))
#place(top + right, float: true, wrap: true, dy: 40pt, clearance: 8pt,
  rect(width: 60pt, height: 70pt, fill: forest))
#lorem(100)

--- wrap-float-rtl paged ---
#set page(width: 220pt, height: 220pt)
#set text(dir: rtl)
#place(top + left, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(60)

--- wrap-float-parent-columns-offset paged ---
#set page(width: 240pt, height: 240pt)
#columns(2)
#place(top + right, float: true, wrap: true, scope: "parent", dx: -6pt, dy: 6pt,
  rect(width: 50pt, height: 60pt, fill: aqua))
#lorem(80)

--- wrap-float-inline-boxes paged ---
#set page(width: 240pt, height: 240pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(20) #box(fill: yellow, inset: 2pt)[Inline box] #lorem(20)

--- wrap-float-math-inline paged ---
#set page(width: 240pt, height: 240pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(10) $ a^2 + b^2 = c^2 $ #lorem(20)

--- wrap-float-code-block paged ---
#set page(width: 240pt, height: 260pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(20)
#code("""
fn main() {
    println!("hello");
}
""")
#lorem(20)
