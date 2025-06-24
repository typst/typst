--- html-non-char html ---
// Error: 1-9 the character `"\u{fdd0}"` cannot be encoded in HTML
\u{fdd0}

--- html-void-element-with-children html ---
// Error: 2-27 HTML void elements must not have children
#html.elem("img", [Hello])

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
// Error: 2-29 HTML raw text element cannot have non-text children
#html.script(html.frame[Ok])

--- html-raw-text-contains-closing-tag html ---
// Error: 2-32 HTML raw text element cannot contain its own closing tag
// Hint: 2-32 the sequence `</SCRiPT` appears in the raw text
#html.script("hello </SCRiPT ")

--- html-escapable-raw-text-contains-closing-tag html ---
// This is okay because we escape it.
#html.textarea("hello </textarea>")
