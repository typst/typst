// Test for loop patterns.
// Ref: false

---
#let out = ()

// Values of array.
#for v in (1, 2, 3) {
    out += (v,)
}

// Values of dictionary.
#for v in (a: 4, b: 5) {
    out += (v,)
}

// Keys and values of dictionary.
#for k, v in (a: 6, b: 7) {
    out += (k,)
    out += (v,)
}

#test(out, (1, 2, 3, 4, 5, "a", 6, "b", 7))

---
// Keys and values of array.
// Error: 6-10 mismatched pattern
#for k, v in (-1, -2, -3) {
    dont-care
}
