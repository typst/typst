--- xml eval ---
// Test reading XML data.
#let data = xml("/assets/data/hello.xml")
#test(data, ((
  tag: "data",
  attrs: (:),
  children: (
    "\n  ",
    (tag: "hello", attrs: (name: "hi"), children: ("1",), namespace: none),
    "\n  ",
    (
      tag: "data",
      attrs: (:),
      children: (
        "\n    ",
        (tag: "hello", attrs: (:), children: ("World",), namespace: none),
        "\n    ",
        (tag: "hello", attrs: (:), children: ("World",), namespace: none),
        "\n  ",
      ),
      namespace: none,
    ),
    "\n",
  ),
  namespace: none,
),))

// Test reading through path type.
#let data-from-path = xml(path("/assets/data/hello.xml"))
#test(data-from-path, data)

--- xml-namespaces eval ---
// Test reading XML data containing namespaces

// TODO: Consider moving this to typst-dev-assets,
//       also see https://github.com/typst/typst-dev-assets/pull/21
#let raw-xml = "
<data xmlns=\"http://example.org\" xmlns:foo=\"urn:foo\">
  <hello name=\"hi\">1</hello>
  <data>
    <foo:hello>World</foo:hello>
    <foo:hello>World</foo:hello>
  </data>
</data>
"

#let data = xml(bytes(raw-xml))
#test(data, ((
  tag: "data",
  attrs: (:),
  children: (
    "\n  ",
    (tag: "hello", attrs: (name: "hi"), children: ("1",), namespace: "http://example.org"),
    "\n  ",
    (
      tag: "data",
      attrs: (:),
      children: (
        "\n    ",
        (tag: "hello", attrs: (:), children: ("World",), namespace: "urn:foo"),
        "\n    ",
        (tag: "hello", attrs: (:), children: ("World",), namespace: "urn:foo"),
        "\n  ",
      ),
      namespace: "http://example.org",
    ),
    "\n",
  ),
  namespace: "http://example.org",
),))

--- xml-invalid eval ---
// Error: "/assets/data/bad.xml" 3:1 failed to parse XML (found closing tag 'data' instead of 'hello')
#xml("/assets/data/bad.xml")

--- xml-decode-deprecated eval ---
// Warning: 14-20 `xml.decode` is deprecated, directly pass bytes to `xml` instead
// Hint: 14-20 it will be removed in Typst 0.15.0
#let _ = xml.decode
