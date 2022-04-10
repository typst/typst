// Test micro-typographical shenanigans.

---
// Test that overhang is off by default in boxes.
A#box["]B

---
// Test justified quotes.
#set par(justify: true)
“A quote that hangs a bit into the margin.” \
  --- somebody

---
// Test fancy quotes in the left margin.
#set par(align: right)
»Book quotes are even smarter.« \
›Book quotes are even smarter.‹ \

---
// Test fancy quotes in the right margin.
#set par(align: left)
«Book quotes are even smarter.» \
‹Book quotes are even smarter.› \

---
#set text(lang: "ar", "Noto Sans Arabic", "IBM Plex Sans")
"المطر هو الحياة" \
المطر هو الحياة

---
// Test that lone punctuation doesn't overhang into the margin.
#set page(margins: 0pt)
#set par(align: right)
:
