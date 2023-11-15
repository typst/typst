// Test that figure captions don't cause panics.
// Ref: false

---
// #2530
#figure(caption: [test])[].caption

---
// #2165
#figure.caption[]

---
// #2328
// Error: 18-43 footnote.entry cannot be constructed
HI#footnote.entry(clearance: 2.5em)[There]

---
// Enum item (pre-emptive)
#enum.item(none)[Hello]
#enum.item(17)[Hello]

---
// List item (pre-emptive)
#list.item[Hello]

---
// Term item (pre-emptive)
#terms.item[Hello][World!]

---
// Outline entry (pre-emptive)
// Error: 2-48 cannot outline text
#outline.entry(1, [Hello], [World!], none, [1])
#outline.entry(1, heading[Hello], [World!], none, [1])
