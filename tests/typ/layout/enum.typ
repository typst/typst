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
// Apply enumitem styles to numbers as well.
+ This is a regular item and has a regular number
#text(blue)[+ This has a blue number]
+ This is a regular item and has a regular number
#hide[+ This is a hidden item, its number is also hidden]
+ The above was a hidden item (with a hidden number)
#strong[+ Bold item and number]

#text(red, enum[This applies red to the entire enum])

1. Should also work
#text(blue)[2. For explicit numbers]
#hide[3. This one is hidden]
4. This is fine

---
// Do not apply styles to numbers when they only apply to the item body.
+ Regular item and number
+ #text(blue)[Blue item body, but regular number]
+ #hide[Hidden item body, but regular number]
+ #strong[Bold item, but regular number]
#text(red)[+ #text(blue)[The item body is blue, but the number is red]]
#list(text(blue)[Number should also be regular here, despite the blue text])

1. #text(blue)[The same applies]
#strong[2. #text(red)[For explicit numbers]]
3. #hide[This is hidden]
4. The above is hidden
