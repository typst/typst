// Test setting custom smartquotes

---
// Use language quotes for missing keys
#set smartquote(quotes: (double: "«»"))
"Double and 'Single' Quotes"

#set smartquote(quotes: (double: ("<[", "]>"), single: "«»"))
"Double and 'Single' Quotes"
