// Test setting custom smartquotes

---
// Use language quotes for missing keys
#set smartquote(double-bikeshed: "«»")
"Double and 'Single' Quotes"

#set smartquote(double-bikeshed: ("<[", "]>"), single: "«»")
"Double and 'Single' Quotes"

---
// Allow 2 graphemes
#set smartquote(single: "a\u{0301}a\u{0301}")
"Double and 'Single' Quotes"

---
// Error: 25-28 expected 2 characters, got 1 character
#set smartquote(single: "'")

---
// Error: 25-35 expected 2 quotes, got 4 quotes
#set smartquote(single: ("'",) * 4)
