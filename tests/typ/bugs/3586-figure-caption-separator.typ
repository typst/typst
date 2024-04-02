// Test that figure caption separator is synthesized correctly.
// https://github.com/typst/typst/issues/3586
// Ref: false

---
#show figure.caption: c => test(c.separator, [#": "])
#figure(table[], caption: [This is a test caption])
