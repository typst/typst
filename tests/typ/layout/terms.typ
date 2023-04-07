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
/ First list: #lorem(6)

#set terms(hanging-indent: 30pt)
/ Second list: #lorem(5)

---
// Test grid like show rule.
#show terms: it => table(
  columns: 2,
  inset: 3pt,
  ..it.children.map(v => (emph(v.term), v.description)).flatten(),
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

---
// Styles applied to whole termitem must apply to term and body.
/ Regular: this is normal.
#text(blue)[/ Blue: This is blue.]
#hide[/ Hidden: This is hidden.]
#emph[/ Emph: the above is hidden.]
#text(red)[/ Red: #text(blue)[Blue.]]

---
// Styles applied to either term or body must not apply to both.
/ #text(blue)[This is blue]: But this is not.
/ #hide[This is hidden]: This is not hidden.
/ This is fine: #hide[This is hidden.]
/ This is normal: #text(red)[This is red.]
/ #text(green)[This is green]: #text(blue)[This is blue.]
