// Test bidirectional text and language configuration.

---
// Test reordering with different top-level paragraph directions.
#let content = par[Text טֶקסט]
#text(lang: "he", content)
#text(lang: "de", content)

---
// Test that consecutive, embedded  LTR runs stay LTR.
// Here, we have two runs: "A" and italic "B".
#let content = par[أنت A#emph[B]مطرC]
#set text(font: ("PT Sans", "Noto Sans Arabic"))
#text(lang: "ar", content)
#text(lang: "de", content)

---
// Test that consecutive, embedded RTL runs stay RTL.
// Here, we have three runs: "גֶ", bold "שֶׁ", and "ם".
#let content = par[Aגֶ#strong[שֶׁ]םB]
#set text(font: ("Linux Libertine", "Noto Serif Hebrew"))
#text(lang: "he", content)
#text(lang: "de", content)

---
// Test embedding up to level 4 with isolates.
#set text(dir: rtl)
א\u{2066}A\u{2067}Bב\u{2069}?

---
// Test hard line break (leads to two paragraphs in unicode-bidi).
#set text(lang: "ar", font: ("Noto Sans Arabic", "PT Sans"))
Life المطر هو الحياة \
الحياة تمطر is rain.

---
// Test spacing.
L #h(1cm) ריווחR \
Lריווח #h(1cm) R

---
// Test inline object.
#set text(lang: "he")
קרנפיםRh#box(image("/files/rhino.png", height: 11pt))inoחיים

---
// Test whether L1 whitespace resetting destroys stuff.
الغالب #h(70pt) ن#" "ة

---
// Test setting a vertical direction.
// Ref: false

// Error: 16-19 text direction must be horizontal
#set text(dir: ttb)
