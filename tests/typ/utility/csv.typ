// Test reading structured CSV data.

---
#set page(width: auto)
#let data = csv("/res/zoo.csv")
#let cells = data(0).map(strong) + data.slice(1).flatten()
#table(columns: data(0).len(), ..cells)

---
// Error: 6-16 file not found (searched at typ/utility/nope.csv)
#csv("nope.csv")

---
// Error: 6-20 failed to load csv file (CSV error: record 2 (line: 3, byte: 8): found record with 3 fields, but the previous record has 2 fields)
#csv("/res/bad.csv")
