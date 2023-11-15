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
// Error: 4-43 footnote entry must have a location
// Hint: 4-43 try using a query or a show rule to customize the footnote instead
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

---
// Outline entry (pre-emptive, improved error)
// Error: 2-55 heading must have a location
// Hint: 2-55 try using a query or a show rule to customize the outline.entry instead
#outline.entry(1, heading[Hello], [World!], none, [1])
