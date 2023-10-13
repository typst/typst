// Test footnotes in columns, even those
// that are not enabled via `set page`.

---
#set page(height: 120pt)
#align(center, strong[Title])
#show: columns.with(2)
#lorem(3) #footnote(lorem(6))
Hello there #footnote(lorem(2))
