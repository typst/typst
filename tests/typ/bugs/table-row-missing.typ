// Test that a table row isn't wrongly treated like a gutter row.

---
#set page(height: 70pt)
#table(
  rows: 16pt,
  ..range(6).map(str).flatten(),
)
