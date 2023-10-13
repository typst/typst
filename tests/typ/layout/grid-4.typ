// Test relative sizing inside grids.

---
// Test that auto and relative columns use the correct base.
#grid(
  columns: (auto, 60%),
  rows: (auto, auto),
  rect(width: 50%, height: 0.5cm, fill: conifer),
  rect(width: 100%, height: 0.5cm, fill: eastern),
  rect(width: 50%, height: 0.5cm, fill: forest),
)

---
// Test that fr columns use the correct base.
#grid(
  columns: (1fr,) * 4,
  rows: (1cm,),
  rect(width: 50%, fill: conifer),
  rect(width: 50%, fill: forest),
  rect(width: 50%, fill: conifer),
  rect(width: 50%, fill: forest),
)

---
// Test that all three kinds of rows use the correct bases.
#set page(height: 4cm, margin: 0cm)
#grid(
  rows: (1cm, 1fr, 1fr, auto),
  rect(height: 50%, width: 100%, fill: conifer),
  rect(height: 50%, width: 100%, fill: forest),
  rect(height: 50%, width: 100%, fill: conifer),
  rect(height: 25%, width: 100%, fill: forest),
)
