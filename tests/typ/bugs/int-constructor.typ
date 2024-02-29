// Test that integer -> integer conversion doesn't do a roundtrip through float.
// Ref: false

---
#let x = 9223372036854775800
#test(type(x), int)
#test(int(x), x)
