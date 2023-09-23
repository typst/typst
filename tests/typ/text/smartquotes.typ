// Test setting custom smartquotes

---
// Use language quotes for missing keys
#set smartquote(double-bikeshed: "«»")
"Double and 'Single' Quotes"

#set smartquote(double-bikeshed: ("<[", "]>"), single: "«»")
"Double and 'Single' Quotes"

---
// Error: 25-30 expected only 2 characters, got unexpected additional character
#set smartquote(single: "'''")

---
// Error: 25-40 expected only 2 quotes, got unexpected additional quote
#set smartquote(single: ("'", "'", "'"))
