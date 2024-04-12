--- repr ---
#test(repr(ltr), "ltr")
#test(repr((1, 2, false, )), "(1, 2, false)")

--- repr-literals ---
// Literal values.
#auto \
#none (empty) \
#true \
#false

--- repr-numerical ---
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
#(100% + (2em + 2pt)) \
#(100% + 0pt) \
#(100% - 2em + 2pt) \
#(100% - 2pt) \
#2.3fr

--- repr-misc ---
// Colors and strokes.
#set text(0.8em)
#rgb("f7a205") \
#(2pt + rgb("f7a205"))

// Strings and escaping.
#raw(repr("hi"), lang: "typc")
#repr("a\n[]\"\u{1F680}string")

// Content.
#raw(lang: "typc", repr[*Hey*]) \
#raw(lang: "typc", repr[A _sequence_]) \
#raw(lang: "typc", repr[A _longer_ *sequence*!])

// Functions.
#let f(x) = x
#f \
#rect \
#(() => none)

// Types.
#int \
#type("hi") \
#type((a: 1))
