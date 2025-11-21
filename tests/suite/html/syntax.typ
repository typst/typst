--- html-void-element-with-children html ---
// Error: 2-27 HTML void elements must not have children
#html.elem("img", [Hello])

--- html-space-collapsing html ---
// Note: <s>..</s> = <span style="white-space: pre-wrap">..</span>
#import html: span

= Single spaces
// No collapsing.
#"A B"
// -> A B

// No collapsing, multiple text elements.
#"A"#" "#"B"
// -> A B

// Across span boundaries: 0-1.
#span[A] B
// -> <span>A</span> B

// With span in between.
#"A "#span()#" B"
// -> A<s> </s><span></span> B

// With metadata in between.
#"A "#metadata(none)#" B"
// -> A<s>  </s>B

// Within span.
#span("A ")B
// -> <span>A </span>B

= Consecutive whitespace
// Single text element.
#"A  B   C"
// -> A<s>  </s>B<s>   </s>C

// Multiple text elements.
A#"  "B#"   C"
// -> A<s>  </s>B<s>   </s>C

// Across span boundaries: 1-1.
#span("A ") B
// -> <span>A<s> </s></span> B

// Across span boundaries: 1-2.
#span("A ")#"  B"
// -> <span>A </span><s>  </s>B

// Across span boundaries: 2-1.
#span("A  ") B
// -> <span>A<s>  </s></span> B

// Across span boundaries: 2-2.
#span("A  ")#"  B"
// -> <span>A<s>  </s></span><s>  </s>B

// With span in between.
#"A  "#span()#"  B"
// -> A<s>  </s><span></span><s>  </s>B

// With metadata in between.
#"A "#metadata(none)#"  B"
// -> A<s>   </s>B

= Leading whitespace
// Leading space.
#" A"
// -> <s> </s>A

// Leading space in span.
#span(" ")A
// -> <span><s> </s></span>A

// Leading space with preceding empty element.
#span()#" "A
// -> <span></span><s> </s>A

= Trailing whitespace
// Trailing space.
#"A "
// -> A<s> </s>

// Trailing space in element.
#span("A ")
// -> A<span><s> </s></span>

// Trailing space in element with following empty element.
#span("A ")#span()
// -> <span>A<s> </s></span><span></span>

= Tabs
// Single text element.
#"A\tB"
// -> A<s>&#9;</s>B

// Multiple text elements.
#"A"#"\t"#"B"
// -> A<s>&#9;</s>B

// Spaces + Tab.
#"A \t B"
// -> A<s> &#9; </s>B

= Newlines
// Normal line feed.
#"A\nB"
// -> A<br>B

// CRLF.
#"A\r\nB"
// -> A<br>B

// Spaces + newline.
#"A \n B"
// -> A<s> </s><br><s> </s>B

// Explicit `<br>` element.
#"A "#html.br()#" B"
// -> A<s> </s><br><s> </s>B

// Newline in span.
#"A "#span("\n")#" B"
// -> A<s> </s><span><br></span><s> </s>B

= With default ignorables
// With default ignorable in between.
#"A \u{200D} B"
// -> A<s> </s>&#x200D; B

#"A  \u{200D}  B"
// -> A<s>  </s>&#x200D;<s>  </s>B

= Everything
// Everything at once.
#span("  A ")#"\r\n\t"B#" "#span()
// -> <span><s>  </s>A<s> </s></span><br><s>&#9;</s>B<s> </s><span></span>

= Special
// Escapable raw.
#html.textarea("A  B")
// -> <textarea>A  B</textarea>

// Preformatted.
#html.pre("A  B")
// -> <pre>A  B</pre>

--- html-pre-starting-with-newline html ---
#html.pre("hello")
#html.pre("\nhello")
#html.pre("\n\nhello")

--- html-textarea-starting-with-newline html ---
#html.textarea("\nenter")

--- html-script html ---
// This should be pretty and indented.
#html.script(
  ```js
  const x = 1
  const y = 2
  console.log(x < y, Math.max(1, 2))
  ```.text,
)

// This should have extra newlines, but no indent because of the multiline
// string literal.
#html.script("console.log(`Hello\nWorld`)")

// This should be untouched.
#html.script(
  type: "text/python",
  ```py
  x = 1
  y = 2
  print(x < y, max(x, y))
  ```.text,
)

--- html-style html ---
// This should be pretty and indented.
#html.style(
  ```css
  body {
    text: red;
  }
  ```.text,
)

--- html-raw-text-contains-elem html ---
// Error: 14-32 HTML raw text element cannot have non-text children
#html.script(html.strong[Hello])

--- html-raw-text-contains-frame html ---
// Error: 14-31 HTML raw text element cannot have non-text children
#html.script(html.frame[Hello])

--- html-raw-text-contains-closing-tag html ---
// Error: 2-32 HTML raw text element cannot contain its own closing tag
// Hint: 2-32 the sequence `</SCRiPT` appears in the raw text
#html.script("hello </SCRiPT ")

--- html-escapable-raw-text-contains-elem html ---
// Error: 16-34 HTML raw text element cannot have non-text children
#html.textarea(html.strong[Hello])

--- html-escapable-raw-text-contains-closing-tag html ---
// This is okay because we escape it.
#html.textarea("hello </textarea>")

--- html-non-char html ---
// Error: 1-9 the character `"\u{fdd0}"` cannot be encoded in HTML
\u{fdd0}

--- html-raw-text-non-char html ---
// Error: 24-32 the character `"\u{fdd0}"` cannot be encoded in HTML
#html.script[const x = \u{fdd0}]

--- html-escapable-raw-text-non-char html ---
// Error: 23-31 the character `"\u{fdd0}"` cannot be encoded in HTML
#html.textarea[Typing \u{fdd0}]
