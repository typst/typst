// Test styles with closure.

---
#set heading(
  size: 10pt,
  around: 0.65em,
  fill: lvl => if even(lvl) { red } else { blue },
)

= Heading 1
== Heading 2
=== Heading 3
==== Heading 4

---
// Test in constructor.
#heading(
  level: 3,
  size: 10pt,
  strong: lvl => {
    assert(lvl == 3)
    false
  }
)[Level 3]

---
// Error: 22-26 expected string or auto or function, found length
#set heading(family: 10pt)
= Heading

---
// Error: 29-38 cannot add integer and string
#set heading(strong: lvl => lvl + "2")
= Heading

---
// Error: 22-34 expected string or auto, found boolean
#set heading(family: lvl => false)
= Heading

---
// Error: 22-37 missing argument: b
#set heading(family: (a, b) => a + b)
= Heading

---
// Error: 22-30 unexpected argument
#set heading(family: () => {})
= Heading
