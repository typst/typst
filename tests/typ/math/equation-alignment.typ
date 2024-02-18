// Test alignment of equation, equation number, and its interaction
// with text direction.

---
// Equation number aligned on the default horizontal alignment
#let eq(eq-alignment) = {
  show math.equation: set align(eq-alignment)
  $ a + b = c $
}

#set math.equation(numbering: "(1)")

#eq(center)
#eq(left)
#eq(right)

#set text(dir: rtl)
#eq(start)
#eq(end)

---
// Test equation alignments, with its number aligned on the start
#let eq(eq-alignment) = {
  show math.equation: set align(eq-alignment)
  $ a + b = c $
}

#set math.equation(numbering: "(1)", number-align: start)

#eq(center)
#eq(left)
#eq(right)

#set text(dir: rtl)
#eq(start)
#eq(end)

---
// Test equation alignments, with its number aligned on the end
#let eq(eq-alignment) = {
  show math.equation: set align(eq-alignment)
  $ a + b = c $
}

#set math.equation(numbering: "(1)", number-align: end)

#eq(center)
#eq(left)
#eq(right)

#set text(dir: rtl)
#eq(start)
#eq(end)

---
// Test equation alignments, with its number aligned on the left
#let eq(eq-alignment) = {
  show math.equation: set align(eq-alignment)
  $ a + b = c $
}

#set math.equation(numbering: "(1)", number-align: left)

#eq(center)
#eq(left)
#eq(right)

#set text(dir: rtl)
#eq(start)
#eq(end)

---
// Test equation alignments, with its number aligned on the right
#let eq(eq-alignment) = {
  show math.equation: set align(eq-alignment)
  $ a + b = c $
}

#set math.equation(numbering: "(1)", number-align: right)

#eq(center)
#eq(left)
#eq(right)

#set text(dir: rtl)
#eq(start)
#eq(end)

---
// Error: 52-58 equation number cannot be `center`-aligned
#set math.equation(numbering: "(1)", number-align: center)
