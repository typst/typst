// Test bidirectional text and language configuration.

---
// Test reordering with different top-level paragraph directions.
#let content = par[Text טֶקסט]
#set text("IBM Plex Serif")
#text(lang: "he", content)
#text(lang: "de", content)

---
// Test that consecutive, embedded  LTR runs stay LTR.
// Here, we have two runs: "A" and italic "B".
#let content = par[أنت A#emph[B]مطرC]
#set text("IBM Plex Serif", "Noto Sans Arabic")
#text(lang: "ar", content)
#text(lang: "de", content)

---
// Test that consecutive, embedded RTL runs stay RTL.
// Here, we have three runs: "גֶ", bold "שֶׁ", and "ם".
#let content = par[Aגֶ#strong[שֶׁ]םB]
#set text("IBM Plex Serif", "Noto Serif Hebrew")
#text(lang: "he", content)
#text(lang: "de", content)

---
// Test embedding up to level 4 with isolates.
#set text(dir: rtl, "IBM Plex Serif")
א\u{2066}A\u{2067}Bב\u{2069}?

---
// Test hard line break (leads to two paragraphs in unicode-bidi).
#set text(lang: "ar", "Noto Sans Arabic", "IBM Plex Serif")
Life المطر هو الحياة \
الحياة تمطر is rain.

---
// Test spacing.
#set text("IBM Plex Serif")
L #h(1cm) ריווחR \
Lריווח #h(1cm) R

---
// Test inline object.
#set text(lang: "he", "IBM Plex Serif")
קרנפיםRh#box(image("/rhino.png", height: 11pt))inoחיים

---
// Test whether L1 whitespace resetting destroys stuff.
الغالب #h(70pt) ن#" "ة

---
// Test setting a vertical direction.
// Ref: false

// Error: 16-19 must be horizontal
#set text(dir: ttb)
