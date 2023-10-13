// Test enum numbering styles.

---
// Test numbering pattern.
#set enum(numbering: "(1.a.*)")
+ First
+ Second
  2. Nested
     + Deep
+ Normal

---
// Test full numbering.
#set enum(numbering: "1.a.", full: true)
+ First
  + Nested

---
// Test numbering with closure.
#enum(
  start: 3,
  spacing: 0.65em - 3pt,
  tight: false,
  numbering: n => text(
    fill: (red, green, blue).at(calc.rem(n, 3)),
    numbering("A", n),
  ),
  [Red], [Green], [Blue], [Red],
)

---
// Test numbering with closure and nested lists.
#set enum(numbering: n => super[#n])
+ A
  + B
+ C

---
// Test numbering with closure and nested lists.
#set text(font: "New Computer Modern")
#set enum(numbering: (..args) => math.mat(args.pos()), full: true)
+ A
  + B
  + C
    + D
+ E
+ F

---
// Error: 22-24 invalid numbering pattern
#set enum(numbering: "")

---
// Error: 22-28 invalid numbering pattern
#set enum(numbering: "(())")
