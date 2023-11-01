// Test that combination of justification and hanging indent doesn't result in
// an underfull first line.

---
#set par(hanging-indent: 2.5cm, justify: true)
#lorem(5)
