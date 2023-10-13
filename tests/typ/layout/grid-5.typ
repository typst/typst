
---
// Test that trailing linebreak doesn't overflow the region.
#set page(height: 2cm)
#grid[
  Hello \
  Hello \
  Hello \

  World
]

---
// Test that broken cell expands vertically.
#set page(height: 2.25cm)
#grid(
  columns: 2,
  gutter: 10pt,
  align(bottom)[A],
  [
    Top
    #align(bottom)[
      Bottom \
      Bottom \
      #v(0pt)
      Top
    ]
  ],
  align(top)[B],
)
