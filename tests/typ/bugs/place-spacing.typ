// Test that placed elements don't add extra block spacing.

---
#show figure: set block(spacing: 4em)

Paragraph before float.
#figure(rect(), placement: bottom)
Paragraph after float.

---
#show place: set block(spacing: 4em)

Paragraph before place.
#place(rect())
Paragraph after place.
