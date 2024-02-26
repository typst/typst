// Test equation number, and its interaction with equation
// block's alignment and text direction.

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
// Error: 52-58 expected `start`, `left`, `right`, or `end`, found center
#set math.equation(numbering: "(1)", number-align: center)

---
// Error: 52-67 expected `start`, `left`, `right`, or `end`, found center
#set math.equation(numbering: "(1)", number-align: center + bottom)

---
#set math.equation(numbering: "(1)")

$ p &= ln a b \
    &= ln a + ln b $

---
#set math.equation(numbering: "(1)", number-align: top+start)

$ p &= ln a b \
    &= ln a + ln b $

---
#show math.equation: set align(left)
#set math.equation(numbering: "(1)", number-align: bottom)

$ q &= ln sqrt(a b) \
    &= 1/2 (ln a + ln b) $

---
// Tests that if the numbering's layout box vertically exceeds the box of
// the equation frame's boundary, the latter's frame is resized correctly
// to encompass the numbering. #box() below delineates the resized frame.
//
// A row with "-" only has a height that's smaller than the height of the
// numbering's layout box. Note we use pattern "1" here, not "(1)", since
// the parenthesis exceeds the numbering's layout box, due to the default
// settings of top-edge and bottom-edge of the TextElem that laid it out.
#set math.equation(numbering: "1", number-align: top)
#box(
$ - &- - \
  a &= b $,
fill: silver)

#set math.equation(numbering: "1", number-align: horizon)
#box(
$ - - - $,
fill: silver)

#set math.equation(numbering: "1", number-align: bottom)
#box(
$ a &= b \
  - &- - $,
fill: silver)
