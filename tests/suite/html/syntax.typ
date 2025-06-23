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
