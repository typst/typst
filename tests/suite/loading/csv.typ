--- csv ---
// Test reading CSV data.
#set page(width: auto)
#let data = csv("/assets/data/zoo.csv")
#let cells = data.at(0).map(strong) + data.slice(1).flatten()
#table(columns: data.at(0).len(), ..cells)

--- csv-row-type-dict ---
// Test reading CSV data with dictionary rows enabled.
#let data = csv("/assets/data/zoo.csv", row-type: dictionary)
#test(data.len(), 3)
#test(data.at(0).Name, "Debby")
#test(data.at(2).Weight, "150kg")
#test(data.at(1).Species, "Tiger")

--- csv-file-not-found ---
// Error: 6-16 file not found (searched at tests/suite/loading/nope.csv)
#csv("nope.csv")

--- csv-invalid ---
// Error: 6-28 failed to parse CSV (found 3 instead of 2 fields in line 3)
#csv("/assets/data/bad.csv")

--- csv-invalid-row-type-dict ---
// Test error numbering with dictionary rows.
// Error: 6-28 failed to parse CSV (found 3 instead of 2 fields in line 3)
#csv("/assets/data/bad.csv", row-type: dictionary)
