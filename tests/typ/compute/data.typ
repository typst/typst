// Test reading structured data and files.
// Ref: false

---
// Test reading plain text files
#let data = read("/hello.txt")
#test(data, "Hello, world!")

---
// Error: 18-32 file not found (searched at /missing.txt)
#let data = read("/missing.txt")

---
// Error: 18-28 file is not valid utf-8
#let data = read("/bad.txt")

---
// Test reading CSV data.
// Ref: true
#set page(width: auto)
#let data = csv("/zoo.csv")
#let cells = data.at(0).map(strong) + data.slice(1).flatten()
#table(columns: data.at(0).len(), ..cells)

---
// Error pattern: 6-16 ^file not found \(searched at typ(\\|/)compute(\\|/)nope.csv\)$
#csv("nope.csv")

---
// Error: 6-16 failed to parse csv file: found 3 instead of 2 fields in line 3
#csv("/bad.csv")

---
// Test reading JSON data.
#let data = json("/zoo.json")
#test(data.len(), 3)
#test(data.at(0).name, "Debby")
#test(data.at(2).weight, 150)

---
// Error: 7-18 failed to parse json file: syntax error in line 3
#json("/bad.json")

---
// Test reading XML data.
#let data = xml("/data.xml")
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
// Error: 6-16 failed to parse xml file: found closing tag 'data' instead of 'hello' in line 3
#xml("/bad.xml")
