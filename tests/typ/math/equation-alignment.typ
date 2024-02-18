// Test alignment of equation, equation number, and its interaction
// with text direction.

---
#set math.equation(numbering: "(1)")

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

---
#set math.equation(numbering: "(1)", number-align: start)

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

---
#set math.equation(numbering: "(1)", number-align: end)

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

---
#set math.equation(numbering: "(1)", number-align: left)

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

---
#set math.equation(numbering: "(1)", number-align: right)

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

---
// Error: 52-58 equation number cannot be `center`-aligned
#set math.equation(numbering: "(1)", number-align: center)
