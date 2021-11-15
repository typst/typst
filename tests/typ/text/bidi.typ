// Test bidirectional text and language configuration.

---
// Test reordering with different top-level paragraph directions.
#let text = [Text טֶקסט]
#font(serif, "Noto Serif Hebrew")
#par(lang: "he") {text}
#par(lang: "de") {text}

---
// Test that consecutive, embedded  LTR runs stay LTR.
// Here, we have two runs: "A" and italic "B".
#let text = [أنت A_B_مطرC]
#font(serif, "Noto Sans Arabic")
#par(lang: "ar") {text}
#par(lang: "de") {text}

---
// Test that consecutive, embedded RTL runs stay RTL.
// Here, we have three runs: "גֶ", bold "שֶׁ", and "ם".
#let text = [Aגֶ*שֶׁ*םB]
#font(serif, "Noto Serif Hebrew")
#par(lang: "he") {text}
#par(lang: "de") {text}

---
// Test embedding up to level 4 with isolates.
#font(serif, "Noto Serif Hebrew", "Twitter Color Emoji")
#par(dir: rtl)
א\u{2066}A\u{2067}Bב\u{2069}?

---
// Test hard line break (leads to two paragraphs in unicode-bidi).
#font("Noto Sans Arabic", serif)
#par(lang: "ar")
Life المطر هو الحياة \
الحياة تمطر is rain.

---
// Test spacing.
#font(serif, "Noto Serif Hebrew")
L #h(1cm) ריווחR \
Lריווח #h(1cm) R

---
// Test inline object.
#font("Noto Serif Hebrew", serif)
#par(lang: "he")
קרנפיםRh#image("../../res/rhino.png", height: 11pt)inoחיים

---
// Test setting a vertical direction.
// Ref: false

// Error: 11-14 must be horizontal
#par(dir: ttb)
