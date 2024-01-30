// Test show-set rules.

---
// Test overriding show-set rules.
#show strong: set text(red)
Hello *World*

#show strong: set text(blue)
Hello *World*

---
// Test show-set rule on the same element.
#set figure(supplement: [Default])
#show figure.where(kind: table): set figure(supplement: [Tableau])
#figure(
  table(columns: 2)[A][B][C][D],
  caption: [Four letters],
)

---
// Test both things at once.
#show heading: set text(red)
= Level 1
== Level 2

#show heading.where(level: 1): set text(blue)
#show heading.where(level: 1): set text(green)
#show heading.where(level: 1): set heading(numbering: "(I)")
= Level 1
== Level 2

---
// Test setting the thing we just matched on.
// This is quite cursed, but it works.
#set heading(numbering: "(I)")
#show heading.where(numbering: "(I)"): set heading(numbering: "1.")
= Heading

---
// Same thing, but even more cursed, because `kind` is synthesized.
#show figure.where(kind: table): set figure(kind: raw)
#figure(table[A], caption: [Code])

---
// Test that show-set rules on the same element don't affect each other. This
// could be implemented, but isn't as of yet.
#show heading.where(level: 1): set heading(numbering: "(I)")
#show heading.where(numbering: "(I)"): set text(red)
= Heading

---
// Test show-set rules on layoutable element to ensure it is realized
// even though it implements `LayoutMultiple`.
#show table: set text(red)
#pad(table(columns: 4)[A][B][C][D])
