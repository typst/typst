--- xml eval ---
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

// Test reading through path type.
#let data-from-path = xml(path("/assets/data/hello.xml"))
#test(data-from-path, data)

--- xml-invalid paged ---
// Error: "/assets/data/bad.xml" 3:0 failed to parse XML (found closing tag 'data' instead of 'hello')
#xml("/assets/data/bad.xml")

--- xml-decode-deprecated eval ---
// Warning: 14-20 `xml.decode` is deprecated, directly pass bytes to `xml` instead
// Hint: 14-20 it will be removed in Typst 0.15.0
#let _ = xml.decode
