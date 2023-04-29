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

---
// Alignment shouldn't affect number
#set align(horizon)

+ ABCDEF\ GHIJKL\ MNOPQR
   + INNER\ INNER\ INNER
+ BACK\ HERE

---
// Enum number alignment
1. a
10. b
100. c

#set enum(number-align: start)
1.  a
8.  b
16.  c

---
// External and number alignment together
#set align(center)
#set enum(number-align: start)

4.  c
8.  d
16. e
   2.  f
   32. g
   64. h
