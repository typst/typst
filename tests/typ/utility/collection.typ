// Test collection functions.
// Ref: false

---
#let memes = "ArE mEmEs gReAt?";
#test(lower(memes), "are memes great?")
#test(upper(memes), "ARE MEMES GREAT?")
#test(upper("Ελλάδα"), "ΕΛΛΆΔΑ")

---
// Test the `len` function.
#test(len(()), 0)
#test(len(("A", "B", "C")), 3)
#test(len("Hello World!"), 12)
#test(len((a: 1, b: 2)), 2)

---
// Error: 5-7 missing argument: collection
#len()

---
// Error: 6-10 expected string, array or dictionary, found length
#len(12pt)

---
// Test the `sorted` function.
#test(sorted(()), ())
#test(sorted((true, false) * 10), (false,) * 10 + (true,) * 10)
#test(sorted(("it", "the", "hi", "text")), ("hi", "it", "text", "the"))
#test(sorted((2, 1, 3, 10, 5, 8, 6, -7, 2)), (-7, 1, 2, 2, 3, 5, 6, 8, 10))

---
// Error: 9-21 cannot compare string with integer
#sorted((1, 2, "ab"))

---
// Error: 9-24 cannot compare template with template
#sorted(([Hi], [There]))
