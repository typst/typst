// Test bidirectional text and language configuration.

---
// Test reordering with different top-level paragraph directions.
#let content = [Text טֶקסט]
#set text(serif, "Noto Serif Hebrew")
#par(lang: "he", content)
#par(lang: "de", content)

---
// Test that consecutive, embedded  LTR runs stay LTR.
// Here, we have two runs: "A" and italic "B".
#let content = [أنت A#emph[B]مطرC]
#set text(serif, "Noto Sans Arabic")
#par(lang: "ar", content)
#par(lang: "de", content)

---
// Test that consecutive, embedded RTL runs stay RTL.
// Here, we have three runs: "גֶ", bold "שֶׁ", and "ם".
#let content = [Aגֶ#strong[שֶׁ]םB]
#set text(serif, "Noto Serif Hebrew")
#par(lang: "he", content)
#par(lang: "de", content)

---
// Test embedding up to level 4 with isolates.
#set text(serif, "Noto Serif Hebrew", "Twitter Color Emoji")
#set par(dir: rtl)
א\u{2066}A\u{2067}Bב\u{2069}?

---
// Test hard line break (leads to two paragraphs in unicode-bidi).
#set text("Noto Sans Arabic", serif)
#set par(lang: "ar")
Life المطر هو الحياة \
الحياة تمطر is rain.

---
// Test spacing.
#set text(serif, "Noto Serif Hebrew")
L #h(1cm) ריווחR \
Lריווח #h(1cm) R

---
// Test inline object.
#set text("Noto Serif Hebrew", serif)
#set par(lang: "he")
קרנפיםRh#image("../../res/rhino.png", height: 11pt)inoחיים

---
// Test setting a vertical direction.
// Ref: false

// Error: 15-18 must be horizontal
#set par(dir: ttb)
