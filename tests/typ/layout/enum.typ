// Test enumerations.

---
#enum[Embrace][Extend][Extinguish]

---
0. Before first!
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
// In the line.
1.2 \
This is 0. \
See 0.3. \

---
// Edge cases.
+
Empty \
+Nope \
a + 0.

---
// Test item number overriding.
1. first
+ second
5. fifth

#enum(
   enum.item(1)[First],
   [Second],
   enum.item(5)[Fifth]
)
