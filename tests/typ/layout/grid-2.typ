// Test using the `grid` function to create a finance table.

---
#set page(width: 11cm, height: 2.5cm)
#grid(
  columns: 5,
  column-gutter: (2fr, 1fr, 1fr),
  row-gutter: 6pt,
  [*Quarter*],
  [Expenditure],
  [External Revenue],
  [Financial ROI],
  [_total_],
  [*Q1*],
  [173,472.57 \$],
  [472,860.91 \$],
  [51,286.84 \$],
  [_350,675.18 \$_],
  [*Q2*],
  [93,382.12 \$],
  [439,382.85 \$],
  [-1,134.30 \$],
  [_344,866.43 \$_],
  [*Q3*],
  [96,421.49 \$],
  [238,583.54 \$],
  [3,497.12 \$],
  [_145,659.17 \$_],
)
