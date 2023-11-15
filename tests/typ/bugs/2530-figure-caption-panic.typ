// Test that figure captions don't cause panics.
// Ref: false

---
// #2530
#figure(caption: [test])[].caption

---
// #2165
#figure.caption[]
