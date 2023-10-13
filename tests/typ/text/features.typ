// Test OpenType features.

---
// Test turning kerning off.
#text(kerning: true)[Tq] \
#text(kerning: false)[Tq]

---
// Test smallcaps.
#smallcaps[Smallcaps]

---
// Test alternates and stylistic sets.
#set text(font: "IBM Plex Serif")
a vs #text(alternates: true)[a] \
ß vs #text(stylistic-set: 5)[ß]

---
// Test ligatures.
fi vs. #text(ligatures: false)[No fi]

---
// Test number type.
#set text(number-type: "old-style")
0123456789 \
#text(number-type: auto)[0123456789]

---
// Test number width.
#text(number-width: "proportional")[0123456789] \
#text(number-width: "tabular")[3456789123] \
#text(number-width: "tabular")[0123456789]

---
// Test extra number stuff.
#set text(font: "IBM Plex Serif")
0 vs. #text(slashed-zero: true)[0] \
1/2 vs. #text(fractions: true)[1/2]

---
// Test raw features.
#text(features: ("smcp",))[Smcp] \
fi vs. #text(features: (liga: 0))[No fi]

---
// Error: 26-31 expected integer or none, found boolean
#set text(stylistic-set: false)

---
// Error: 26-28 stylistic set must be between 1 and 20
#set text(stylistic-set: 25)

---
// Error: 24-25 expected "lining", "old-style", or auto, found integer
#set text(number-type: 2)

---
// Error: 21-26 expected array or dictionary, found boolean
#set text(features: false)

---
// Error: 21-35 expected string, found boolean
#set text(features: ("tag", false))
