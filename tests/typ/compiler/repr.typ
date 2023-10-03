// Test representation of values in the document.

---
// Literal values.
#auto \
#none (empty) \
#true \
#false

---
// Numerical values.
#1 \
#1.0e-4 \
#3.15 \
#1e-10 \
#50.368% \
#0.0000012345pt \
#4.5cm \
#12e1pt \
#2.5rad \
#45deg \
#1.7em \
#(1cm + 0em) \
#(2em + 10pt) \
#2.3fr

---
// Colors and strokes.
#set text(0.8em)
#rgb("f7a205") \
#(2pt + rgb("f7a205"))

// Strings and escaping.
#raw(repr("hi"), lang: "typc")
#repr("a\n[]\"\u{1F680}string")

// Content.
#raw(lang: "typc", repr[*Hey*])

// Functions.
#let f(x) = x
#f \
#rect \
#(() => none)

// Types.
#int \
#type("hi") \
#type((a: 1))
