// Test reading structured data and files.
// Ref: false

---
// Test reading plain text files
#let data = read("/assets/text/hello.txt")
#test(data, "Hello, world!\n")

---
// Error: 18-44 file not found (searched at assets/text/missing.txt)
#let data = read("/assets/text/missing.txt")

---
// Error: 18-40 file is not valid utf-8
#let data = read("/assets/text/bad.txt")

---
// Test reading CSV data.
// Ref: true
#set page(width: auto)
#let data = csv("/assets/data/zoo.csv")
#let cells = data.at(0).map(strong) + data.slice(1).flatten()
#table(columns: data.at(0).len(), ..cells)

---
// Test reading CSV data with dictionary rows enabled.
#let data = csv("/assets/data/zoo.csv", row-type: dictionary)
#test(data.len(), 3)
#test(data.at(0).Name, "Debby")
#test(data.at(2).Weight, "150kg")
#test(data.at(1).Species, "Tiger")

---
// Error: 6-16 file not found (searched at typ/compute/nope.csv)
#csv("nope.csv")

---
// Error: 6-28 failed to parse CSV (found 3 instead of 2 fields in line 3)
#csv("/assets/data/bad.csv")

---
// Test error numbering with dictionary rows.
// Error: 6-28 failed to parse CSV (found 3 instead of 2 fields in line 3)
#csv("/assets/data/bad.csv", row-type: dictionary)

---
// Test reading JSON data.
#let data = json("/assets/data/zoo.json")
#test(data.len(), 3)
#test(data.at(0).name, "Debby")
#test(data.at(2).weight, 150)

---
// Error: 7-30 failed to parse JSON (expected value at line 3 column 14)
#json("/assets/data/bad.json")

---
// Test reading TOML data.
#let data = toml("/assets/data/toml-types.toml")
#test(data.string, "wonderful")
#test(data.integer, 42)
#test(data.float, 3.14)
#test(data.boolean, true)
#test(data.array, (1, "string", 3.0, false))
#test(data.inline_table, ("first": "amazing", "second": "greater") )
#test(data.table.element, 5)
#test(data.table.others, (false, "indeed", 7))
#test(data.date_time, datetime(
  year: 2023,
  month: 2,
  day: 1,
  hour: 15,
  minute: 38,
  second: 57,
))
#test(data.date_time2, datetime(
  year: 2023,
  month: 2,
  day: 1,
  hour: 15,
  minute: 38,
  second: 57,
))
#test(data.date, datetime(
  year: 2023,
  month: 2,
  day: 1,
))
#test(data.time, datetime(
  hour: 15,
  minute: 38,
  second: 57,
))

---
// Error: 7-30 failed to parse TOML (expected `.`, `=` at line 1 column 16)
#toml("/assets/data/bad.toml")

---
// Test reading YAML data
#let data = yaml("/assets/data/yaml-types.yaml")
#test(data.len(), 9)
#test(data.null_key, (none, none))
#test(data.string, "text")
#test(data.integer, 5)
#test(data.float, 1.12)
#test(data.mapping, ("1": "one", "2": "two"))
#test(data.seq, (1,2,3,4))
#test(data.bool, false)
#test(data.keys().contains("true"), true)
#test(data.at("1"), "ok")

---
// Error: 7-30 failed to parse YAML (did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 18)
#yaml("/assets/data/bad.yaml")

---
// Test reading XML data.
#let data = xml("/assets/data/hello.xml")
#test(data, ((
  tag: "data",
  attrs: (:),
  children: (
    "\n  ",
    (tag: "hello", attrs: (name: "hi"), children: ("1",)),
    "\n  ",
    (
      tag: "data",
      attrs: (:),
      children: (
        "\n    ",
        (tag: "hello", attrs: (:), children: ("World",)),
        "\n    ",
        (tag: "hello", attrs: (:), children: ("World",)),
        "\n  ",
      ),
    ),
    "\n",
  ),
),))

---
// Error: 6-28 failed to parse XML (found closing tag 'data' instead of 'hello' in line 3)
#xml("/assets/data/bad.xml")
