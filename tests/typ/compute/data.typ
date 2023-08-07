// Test reading structured data and files.
// Ref: false

---
// Test reading plain text files
#let data = read("/files/hello.txt")
#test(data, "Hello, world!\n")

---
// Error: 18-38 file not found (searched at files/missing.txt)
#let data = read("/files/missing.txt")

---
// Error: 18-34 file is not valid utf-8
#let data = read("/files/bad.txt")

---
// Test reading CSV data.
// Ref: true
#set page(width: auto)
#let data = csv("/files/zoo.csv")
#let cells = data.at(0).map(strong) + data.slice(1).flatten()
#table(columns: data.at(0).len(), ..cells)

---
// Error: 6-16 file not found (searched at typ/compute/nope.csv)
#csv("nope.csv")

---
// Error: 6-22 failed to parse csv file: found 3 instead of 2 fields in line 3
#csv("/files/bad.csv")

---
// Test reading JSON data.
#let data = json("/files/zoo.json")
#test(data.len(), 3)
#test(data.at(0).name, "Debby")
#test(data.at(2).weight, 150)

---
// Error: 7-24 failed to parse json file: syntax error in line 3
#json("/files/bad.json")

---
// Test reading TOML data.
#let data = toml("/files/toml-types.toml")
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

---
// Error: 7-24 failed to parse toml file: expected `.`, `=`, index 15-16
#toml("/files/bad.toml")

---
// Test reading YAML data
#let data = yaml("/files/yaml-types.yaml")
#test(data.len(), 7)
#test(data.null_key, (none, none))
#test(data.string, "text")
#test(data.integer, 5)
#test(data.float, 1.12)
#test(data.mapping, ("1": "one", "2": "two"))
#test(data.seq, (1,2,3,4))
#test(data.bool, false)
#test(data.keys().contains("true"), false)
---

---
// Error: 7-24 failed to parse yaml file: while parsing a flow sequence, expected ',' or ']' at line 2 column 1
#yaml("/files/bad.yaml")

---
// Test reading XML data.
#let data = xml("/files/data.xml")
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
// Error: 6-22 failed to parse xml file: found closing tag 'data' instead of 'hello' in line 3
#xml("/files/bad.xml")
