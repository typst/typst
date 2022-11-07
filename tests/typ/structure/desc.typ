// Test description lists.

---
/
No: list \
/No: list

---
// Test with constructor.
#desc(
  (term: [One], body: [First]),
  (term: [Two], body: [Second]),
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
#set desc(body-indent: 30pt)
/ Second list: #lorem(4)

---
// Test grid like show rule.
#show desc: it => table(
  columns: 2,
  padding: 3pt,
  ..it.items.map(item => (emph(item.term), item.body)).flatten(),
)

/ A: One letter
/ BB: Two letters
/ CCC: Three letters
