// Test bidirectional text and language configuration.

---
// Test reordering with different top-level paragraph directions.
#let text = [Text טֶקסט]
#font("EB Garamond", "Noto Serif Hebrew")
#lang("he") {text}
#lang("de") {text}

---
// Test that consecutive, embedded  LTR runs stay LTR.
// Here, we have two runs: "A" and italic "B".
#let text = [أنت A_B_مطرC]
#font("EB Garamond", "Noto Sans Arabic")
#lang("ar") {text}
#lang("de") {text}

---
// Test that consecutive, embedded RTL runs stay RTL.
// Here, we have three runs: "גֶ", bold "שֶׁ", and "ם".
#let text = [Aגֶ*שֶׁ*םB]
#font("EB Garamond", "Noto Serif Hebrew")
#lang("he") {text}
#lang("de") {text}

---
// Test embedding up to level 4 with isolates.
#font("EB Garamond", "Noto Serif Hebrew", "Twitter Color Emoji")
#lang(dir: rtl)
א\u{2066}A\u{2067}Bב\u{2069}?

---
// Test hard line break (leads to two paragraphs in unicode-bidi).
#font("Noto Sans Arabic", "EB Garamond")
#lang("ar")
Life المطر هو الحياة \
الحياة تمطر is rain.

---
// Test spacing.
#font("EB Garamond", "Noto Serif Hebrew")
L #h(1cm) ריווחR \
Lריווח #h(1cm) R

---
// Test inline object.
#font("Noto Serif Hebrew", "EB Garamond")
#lang("he")
קרנפיםRh#image("../../res/rhino.png", height: 11pt)inoחיים

---
// Test the `lang` function.
// Ref: false

// Error: 12-15 must be horizontal
#lang(dir: ttb)
