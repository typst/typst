// Test inline equation line breaking.

---
// Basic breaking after binop, rel
#let hrule(x) = box(line(length: x))
#hrule(45pt)$e^(pi i)+1 = 0$\
#hrule(55pt)$e^(pi i)+1 = 0$\
#hrule(70pt)$e^(pi i)+1 = 0$

---
// LR groups prevent linbreaking.
#let hrule(x) = box(line(length: x))
#hrule(76pt)$a+b$\
#hrule(74pt)$(a+b)$\
#hrule(74pt)$paren.l a+b paren.r$

---
// Multiline yet inline does not linebreak
#let hrule(x) = box(line(length: x))
#hrule(80pt)$a + b \ c + d$\

---
// A single linebreak at the end still counts as one line.
#let hrule(x) = box(line(length: x))
#hrule(60pt)$e^(pi i)+1 = 0\ $

---
// Inline, in a box, doesn't linebreak.
#let hrule(x) = box(line(length: x))
#hrule(80pt)#box($a+b$)

---
// A relation followed by a relation doesn't linebreak
#let hrule(x) = box(line(length: x))
#hrule(70pt)$a < = b$\
#hrule(74pt)$a < = b$

---
// Page breaks can happen after a relation even if there is no 
// explicit space.
#let hrule(x) = box(line(length: x))
#hrule(90pt)$<;$\
#hrule(95pt)$<;$\
#hrule(90pt)$<)$\
#hrule(95pt)$<)$

---
// Verify empty rows are handled ok.
$ $\
Nothing: $ $, just empty.
