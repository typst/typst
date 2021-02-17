// Test which things are iterable.
// Ref: false

---
// Array.

#for x in () {}

#let sum = 0
#for x in (1, 2, 3, 4, 5) {
    sum += x
}

#test(sum, 15)

---
// Dictionary.
// Ref: true
(\ #for k, v in (name: "Typst", age: 2) [
    #h(0.5cm) {k}: {v}, \
])

---
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
// Uniterable expression.
// Error: 11-15 cannot loop over boolean
#for v in true {}

// Make sure that we don't complain twice.
// Error: 11-18 cannot add integer and string
#for v in 1 + "2" {}

// Error: 14-17 cannot apply '-' to string
#let error = -""
#let result = for v in (1, 2, 3) {
    if v < 2 [Ok] else {error}
}
#test(result, error)
