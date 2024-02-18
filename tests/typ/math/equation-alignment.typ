// Test alignment of equation, equation number, and its interaction
// with text direction.

---
// Equation alignment: mutate through possible values.
// Number alignment: use the default setting.
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
// Equation alignment: mutate through possible values.
// Number alignment: start of block, respecting text direction.
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
// Equation alignment: mutate through possible values.
// Number alignment: end of block, respecting text direction.
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
// Equation alignment: mutate through possible values.
// Number alignment: left of block.
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
// Equation alignment: mutate through possible values.
// Number alignment: right of block.
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
