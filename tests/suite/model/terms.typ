// Test term list.

--- terms-constructor ---
// Test with constructor.
#terms(
  ([One], [First]),
  ([Two], [Second]),
)

--- terms-built-in-loop ---
// Test joining.
#for word in lorem(4).split().map(s => s.trim(".")) [
  / #word: Latin stuff.
]

--- terms-multiline ---
// Test multiline.
#set text(8pt)

/ Fruit: A tasty, edible thing.
/ Veggie:
  An important energy source
  for vegetarians.

  And healthy!

--- terms-style-change-interrupted ---
// Test style change.
#set text(8pt)
/ First list: #lorem(6)

#set terms(hanging-indent: 30pt)
/ Second list: #lorem(5)

--- terms-rtl ---
// Test RTL.
#set text(8pt, dir: rtl)

/ פרי: דבר טעים, אכיל. ומקור אנרגיה חשוב לצמחונים.

--- terms-grid ---
// Test grid like show rule.
#show terms: it => table(
  columns: 2,
  inset: 3pt,
  ..it.children.map(v => (emph(v.term), v.description)).flatten(),
)

/ A: One letter
/ BB: Two letters
/ CCC: Three letters

--- terms-syntax-edge-cases ---
/ Term:
Not in list
/Nope

--- terms-missing-colon ---
// Error: 8 expected colon
/ Hello

--- issue-1050-terms-indent ---
#set page(width: 200pt)
#set par(first-line-indent: 0.5cm)

- #lorem(10)
- #lorem(10)

+ #lorem(10)
+ #lorem(10)

/ Term 1: #lorem(10)
/ Term 2: #lorem(10)

--- issue-2530-term-item-panic ---
// Term item (pre-emptive)
#terms.item[Hello][World!]
