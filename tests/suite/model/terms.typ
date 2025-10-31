// Test term list.

--- terms-constructor render pdftags ---
// Test with constructor.
#terms(
  ([One], [First]),
  ([Two], [Second]),
)

--- terms-built-in-loop render ---
// Test joining.
#for word in lorem(4).split().map(s => s.trim(".")) [
  / #word: Latin stuff.
]

--- terms-multiline render ---
// Test multiline.
#set text(8pt)

/ Fruit: A tasty, edible thing.
/ Veggie:
  An important energy source
  for vegetarians.

  And healthy!

--- terms-style-change-interrupted render ---
// Test style change.
#set text(8pt)
/ First list: #lorem(6)

#set terms(hanging-indent: 30pt)
/ Second list: #lorem(5)

--- terms-rtl render ---
// Test RTL.
#set text(8pt, dir: rtl)

/ פרי: דבר טעים, אכיל. ומקור אנרגיה חשוב לצמחונים.

--- terms-grid render ---
// Test grid like show rule.
#show terms: it => table(
  columns: 2,
  inset: 3pt,
  ..it.children.map(v => (emph(v.term), v.description)).flatten(),
)

/ A: One letter
/ BB: Two letters
/ CCC: Three letters

--- terms-syntax-edge-cases render ---
/ Term:
Not in list
/Nope

--- terms-missing-colon render ---
// Error: 8 expected colon
/ Hello

--- terms-par render html ---
// Check whether the contents of term list items become paragraphs.
#show par: it => if target() != "html" { highlight(it) } else { it }

// No paragraphs.
#block[
  / Hello: A
  / World: B
]

#block[
  / Hello: A // Paragraphs

    From
  / World: B // No paragraphs because it's a tight term list.
]

#block[
  / Hello: A // Paragraphs

    From

    The

  / World: B // Paragraph because it's a wide term list.
]


--- issue-1050-terms-indent render ---
#set page(width: 110pt)
#set par(first-line-indent: 0.5cm)

- #lorem(5)
- #lorem(5)

+ #lorem(5)
+ #lorem(5)

/ S: #lorem(5)
/ XXXL: #lorem(5)

--- issue-2530-term-item-panic render ---
// Term item (pre-emptive)
#terms.item[Hello][World!]

--- issue-5503-terms-in-align render ---
// `align` is block-level and should interrupt a `terms`.
#show terms: [Terms]
/ a: a
#align(right)[/ i: i]
/ j: j

--- issue-5719-terms-nested render ---
// Term lists can be immediately nested.
/ Term A: 1
/ Term B: / Term C: 2
          / Term D: 3
