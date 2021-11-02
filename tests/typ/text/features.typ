// Test OpenType features.

---
// Test turning kerning off.
#font(kerning: true)[Tq] \
#font(kerning: false)[Tq]

---
// Test smallcaps.
#font("Roboto")
#font(smallcaps: true)[Smallcaps]

---
// Test alternates and stylistic sets.
#font("IBM Plex Serif")
a vs #font(alternates: true)[a] \
ß vs #font(stylistic-set: 5)[ß]

---
// Test ligatures.
fi vs. #font(ligatures: false)[No fi] \

---
// Test number type.
#font("Roboto")
#font(number-type: "old-style") 0123456789 \
#font(number-type: auto)[0123456789]

---
// Test number width.
#font("Roboto")
#font(number-width: "proportional")[0123456789] \
#font(number-width: "tabular")[3456789123] \
#font(number-width: "tabular")[0123456789]

---
// Test number position.
#font("IBM Plex Sans")
#font(number-position: "normal")[C2H4] \
#font(number-position: "subscript")[C2H4] \
#font(number-position: "superscript")[C2H4]

---
// Test extra number stuff.
#font("IBM Plex Sans")
0 vs. #font(slashed-zero: true)[0] \
1/2 vs. #font(fractions: true)[1/2]

---
// Test raw features.
#font("Roboto")
#font(features: ("smcp",))[Smcp] \
fi vs. #font(features: (liga: 0))[No fi]

---
// Error: 22-24 must be between 1 and 20
#font(stylistic-set: 25)

---
// Error: 20-31 expected "lining" or "old-style"
#font(number-type: "different")

---
// Error: 17-22 expected array of strings or dictionary mapping tags to integers, found boolean
#font(features: false)
