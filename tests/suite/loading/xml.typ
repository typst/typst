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
// Error: 6-28 failed to parse XML (found closing tag 'data' instead of 'hello' in line 3)
#xml("/assets/data/bad.xml")

--- xml-decode-deprecated ---
// Warning: 14-20 `xml.decode` is deprecated, directly pass bytes to `xml` instead
#let _ = xml.decode
