// Test setting custom smartquotes

---
// Use language quotes for missing keys
#set smartquote(double-bikeshed: "«»")
"Double and 'Single' Quotes"

#set smartquote(double-bikeshed: ("<[", "]>"), single: "«»")
"Double and 'Single' Quotes"
