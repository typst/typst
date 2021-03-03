// Test for loops.

---
// Empty array.
#for x in () [Nope]

// Array.
#let sum = 0
#for x in (1, 2, 3, 4, 5) {
    sum += x
}

#test(sum, 15)

// Dictionary is not traversed in insertion order.
// Should output `age: 1, name: Typst,`.
#for k, v in (name: "Typst", age: 2) [
    {k}: {v}, \
]

// String.
{
    let out = ""
    let first = true
    for c in "abc" {
        if not first {
            out += ", "
        }
        first = false
        out += c
    }
    test(out, "a, b, c")
}

---
// Block body.
// Should output `[1st, 2nd, 3rd, 4th, 5th, 6th]`.
{
    "[" + for v in (1, 2, 3, 4, 5, 6) {
        (if v > 1 [, ]
            + [{v}]
            + if v == 1 [st]
            + if v == 2 [nd]
            + if v == 3 [rd]
            + if v >= 4 [th])
     } + "]"
}

// Template body.
// Should output `234`.
#for v in (1, 2, 3, 4, 5, 6, 7) [#if v >= 2 and v <= 5 { repr(v) }]

---
// Value of for loops.
// Ref: false
#test(type(for v in () {}), "template")
#test(type(for v in () []), "template")

---
// Uniterable expression.
// Error: 11-15 cannot loop over boolean
#for v in true {}

// Make sure that we don't complain twice.
// Error: 11-18 cannot add integer and string
#for v in 1 + "2" {}

// A single error stops iteration.
#test(error, for v in (1, 2, 3) {
    if v < 2 [Ok] else {error}
})
