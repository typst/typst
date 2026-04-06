--- xml eval ---
// Test reading XML data.
#let data = xml("/assets/data/hello.xml")
#test(data, ((
  namespace: none,
  tag: "data",
  attrs: (:),
  children: (
    "\n  ",
    (namespace: none, tag: "hello", attrs: (name: "hi"), children: ("1",)),
    "\n  ",
    (
      namespace: none,
      tag: "data",
      attrs: (:),
      children: (
        "\n    ",
        (namespace: none, tag: "hello", attrs: (:), children: ("World",)),
        "\n    ",
        (namespace: none, tag: "hello", attrs: (:), children: ("World",)),
        "\n  ",
      ),
    ),
    "\n",
  ),
),))

// Test reading through path type.
#let data-from-path = xml(path("/assets/data/hello.xml"))
#test(data-from-path, data)

--- xml-namespaces eval ---
// Test reading XML data containing namespaces.
#test(
  xml(bytes(
    ```xml
    <data xmlns="http://example.org" xmlns:foo="urn:foo">
      <hello name="hi">1</hello>
      <foo:hello>World</foo:hello>
    </data>
    ```.text
  )),
  ((
    namespace: "http://example.org",
    tag: "data",
    attrs: (:),
    children: (
      "\n  ",
      (namespace: "http://example.org", tag: "hello", attrs: (name: "hi"), children: ("1",)),
      "\n  ",
      (namespace: "urn:foo", tag: "hello", attrs: (:), children: ("World",)),
      "\n",
    ),
  ),),
)

--- xml-invalid eval ---
// Error: "/assets/data/bad.xml" 3:1 failed to parse XML (found closing tag 'data' instead of 'hello')
#xml("/assets/data/bad.xml")

--- xml-decode-deprecated eval ---
// Warning: 14-20 `xml.decode` is deprecated, directly pass bytes to `xml` instead
// Hint: 14-20 it will be removed in Typst 0.15.0
#let _ = xml.decode
