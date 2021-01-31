// Test unary expressions.
// Ref: false

---
// Test plus and minus.
#for v #in (1, 3.14, 12pt, 45deg, 90%, 13% + 10pt) {
    // Test plus.
    test(+v, v)

    // Test minus.
    test(-v, -1 * v)
    test(--v, v)

    // Test combination.
    test(-++ --v, -v)
}

#[test -(4 + 2), 6-12]

---
// Test not.
#[test not true, false]
#[test not false, true]
