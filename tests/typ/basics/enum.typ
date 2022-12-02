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
#set enum(label: "~ A:")
1. First
 + Second

#set enum(label: "(*)")
+ A
+ B
+ C

#set enum(label: "i)")
+ A
+ B

---
// Mix of different lists
- List
+ Enum
/ Desc: List

---
// Test label closure.
#enum(
   start: 4,
   spacing: 0.65em - 3pt,
   tight: false,
   label: n => text(fill: (red, green, blue)(mod(n, 3)), numbering("A", n)),
   [Red], [Green], [Blue],
)

---
#set enum(label: n => n > 1)
+ A
+ B

---
// Lone plus is not an enum.
+
No enum

---
// Error: 18-20 invalid numbering pattern
#set enum(label: "")

---
// Error: 18-24 invalid numbering pattern
#set enum(label: "(())")
