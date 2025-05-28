--- html ---
// Test reading XML data.
#let data = html-decode("/assets/text/example.html")
#test(data, ((
  tag: "html",
  attrs: (:),
  children: (
    (
      tag: "head",
      attrs: (:),
      children: (
        "\n    ",
        (
          tag: "meta",
          attrs: (charset: "UTF-8"),
          children: (),
        ),
        "\n    ",
        (
          tag: "title",
          attrs: (:),
          children: ("Example document",),
        ),
        "\n  ",
      ),
    ),
    "\n  ",
    (
      tag: "body",
      attrs: (:),
      children: (
        "\n    ",
        (
          tag: "h1",
          attrs: (:),
          children: ("Hello, world!",),
        ),
        "\n  \n\n",
      ),
    ),
  ),
),))

--- html-invalid ---
// Error: 14-38 failed to parse HTML (Unexpected token)
#html-decode("/assets/text/hello.txt")
