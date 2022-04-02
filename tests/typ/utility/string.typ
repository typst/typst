// Test string related methods.
// Ref: false

---
// Test conversion to string.
#test(str(123), "123")
#test(str(50.14), "50.14")
#test(str(10 / 3).len() > 10, true)
#test(repr(ltr), "ltr")
#test(repr((1, 2, false, )), "(1, 2, false)")

---
// Error: 6-8 cannot convert content to string
#str([])

---
// Test the `split` and `trim` methods.
#test(
    "Typst, LaTeX, Word, InDesign".split(",").map(s => s.trim()),
    ("Typst", "LaTeX", "Word", "InDesign"),
)

---
// Test the `upper` and `lower` functions.
#let memes = "ArE mEmEs gReAt?";
#test(lower(memes), "are memes great?")
#test(upper(memes), "ARE MEMES GREAT?")
#test(upper("Ελλάδα"), "ΕΛΛΆΔΑ")

---
// Error: 8-9 expected string or content, found integer
#upper(1)

---
// Error: 9-11 must be at least zero
#symbol(-1)

---
// Test integrated lower, upper and symbols.
// Ref: true

#upper("Abc 8")
#upper[def]

#lower("SCREAMING MUST BE SILENCED in " + roman(1672) + " years")

#for i in range(9) {
    symbol(i)
    [ and ]
    roman(i)
    [ for #i]
    parbreak()
}
