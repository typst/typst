
---
// Test that trailing linebreak doesn't overflow the region.
#page(height: 2cm)
#grid[
  Hello \
  Hello \
  Hello \

  World
]

---
// Test that broken cell expands vertically.
#page(height: 2.25cm)
#grid(
  columns: 2,
  gutter: 10pt,
  [#align(bottom) A],
  [
    Top
    #align(bottom)
    Bottom \
    Bottom \
    Top
  ],
  [#align(top) B],
)
