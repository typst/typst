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
// Mix of different lists
- Bullet List
+ Numbered List
/ Term: List

---
// Edge cases.
+
Empty
+Nope
