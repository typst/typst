--- xml ---
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

--- xml-invalid ---
// Error: "/assets/data/bad.xml" 3:0 failed to parse XML (found closing tag 'data' instead of 'hello')
#xml("/assets/data/bad.xml")

--- xml-decode-deprecated ---
// Warning: 14-20 `xml.decode` is deprecated, directly pass bytes to `xml` instead
#let _ = xml.decode
