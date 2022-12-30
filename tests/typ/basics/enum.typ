// Test enumerations.

---
#enum[Embrace][Extend][Extinguish]

---
1. First.
   2. Indented

+ Second

---
// Test automatic numbering in summed content.
#for i in range(5) {
   [+ #numbering("I", 1 + i)]
}

---
// Test label pattern.
#set enum(numbering: "~ A:")
1. First
 + Second

#set enum(numbering: "(*)")
+ A
+ B
+ C

#set enum(numbering: "i)")
+ A
+ B

---
// Mix of different lists
- Bullet List
+ Numbered List
/ Term: List

---
// Test numbering with closure.
#enum(
  start: 4,
  spacing: 0.65em - 3pt,
  tight: false,
  numbering: n => text(
    fill: (red, green, blue).at(mod(n, 3)),
    numbering("A", n),
  ),
  [Red], [Green], [Blue],
)

---
#set enum(numbering: n => n > 1)
+ A
+ B

---
// Lone plus is not an enum.
+
No enum

---
// Error: 22-24 invalid numbering pattern
#set enum(numbering: "")

---
// Error: 22-28 invalid numbering pattern
#set enum(numbering: "(())")
