// Test shape fill & stroke for specific values that used to make the stroke
// deformed.
// https://github.com/typst/typst/issues/3700

---
#rect(
  radius: 1mm,
  width: 100%,
  height: 10pt,
  stroke: (left: rgb("46b3c2") + 16.0mm),
)