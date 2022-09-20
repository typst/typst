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
// Error: 6-20 failed to parse csv file: found 3 instead of 2 fields in line 3
#csv("/res/bad.csv")
