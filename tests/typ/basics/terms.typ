// Test term list.

---
// Test with constructor.
#terms(
  ([One], [First]),
  ([Two], [Second]),
)

---
// Test joining.
#for word in lorem(4).split().map(s => s.trim(".")) [
  / #word: Latin stuff.
]

---
// Test multiline.
#set text(8pt)

/ Fruit: A tasty, edible thing.
/ Veggie:
  An important energy source
  for vegetarians.

---
// Test style change.
#set text(8pt)
/ First list: #lorem(4)

#set terms(hanging-indent: 30pt)
/ Second list: #lorem(4)

---
// Test grid like show rule.
#show terms: it => table(
  columns: 2,
  inset: 3pt,
  ..it.items.map(item => (emph(item.at(0)), item.at(1))).flatten(),
)

/ A: One letter
/ BB: Two letters
/ CCC: Three letters

---
/ Term:
Not in list
/Nope

---
// Error: 8 expected colon
/ Hello
