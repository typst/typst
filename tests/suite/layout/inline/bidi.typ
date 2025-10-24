// Test bidirectional text and language configuration.

--- bidi-en-he-top-level render ---
// Test reordering with different top-level paragraph directions.
#let content = par[Text טֶקסט]
#text(lang: "he", content)
#text(lang: "de", content)

--- bidi-consecutive-embedded-ltr-runs render ---
// Test that consecutive, embedded LTR runs stay LTR.
// Here, we have two runs: "A" and italic "B".
#let content = par[أنت A#emph[B]مطرC]
#set text(font: ("PT Sans", "Noto Sans Arabic"))
#text(lang: "ar", content)
#text(lang: "de", content)

--- bidi-consecutive-embedded-rtl-runs render ---
// Test that consecutive, embedded RTL runs stay RTL.
// Here, we have three runs: "גֶ", bold "שֶׁ", and "ם".
#let content = par[Aגֶ#strong[שֶׁ]םB]
#set text(font: ("Libertinus Serif", "Noto Serif Hebrew"))
#text(lang: "he", content)
#text(lang: "de", content)

--- bidi-nesting render ---
// Test embedding up to level 4 with isolates.
#set text(dir: rtl)
א\u{2066}A\u{2067}Bב\u{2069}?

--- bidi-manual-linebreak render ---
// Test hard line break (leads to two paragraphs in unicode-bidi).
#set text(lang: "ar", font: ("Noto Sans Arabic", "PT Sans"))
Life المطر هو الحياة \
الحياة تمطر is rain.

--- bidi-spacing render ---
// Test spacing.
L #h(1cm) ריווחR \
Lריווח #h(1cm) R

--- bidi-obj render ---
// Test inline object.
#set text(lang: "he")
קרנפיםRh#box(image("/assets/images/rhino.png", height: 11pt))inoחיים

--- bidi-whitespace-reset render ---
// Test whether L1 whitespace resetting destroys stuff.
#set text(font: ("Libertinus Serif", "Noto Sans Arabic"))
الغالب #h(70pt) ن#" "ة

--- bidi-explicit-dir render ---
// Test explicit dir
#set text(dir: rtl)
#text("8:00 - 9:00", dir: ltr) בבוקר
#linebreak()
ב #text("12:00 - 13:00", dir: ltr) בצהריים

--- bidi-raw render ---
// Mixing raw
#set text(lang: "he")
לדוג. `if a == b:` זה תנאי
#set raw(lang: "python")
לדוג. `if a == b:` זה תנאי

#show raw: set text(dir:rtl)
לתכנת בעברית `אם א == ב:`

--- bidi-vertical render ---
// Test setting a vertical direction.
// Error: 16-19 text direction must be horizontal
#set text(dir: ttb)

--- issue-1373-bidi-tofus render ---
// Test that shaping missing characters in both left-to-right and
// right-to-left directions does not cause a crash.
#"\u{590}\u{591}\u{592}\u{593}"

#"\u{30000}\u{30001}\u{30002}\u{30003}"

--- issue-5490-bidi-invalid-range render ---
#set text(lang: "he")
#set raw(lang: "python")
#set page(width: 240pt)
בדיקה האם מספר מתחלק במספר אחר. לדוגמה `if a % 2 == 0`

--- issue-5490-bidi-invalid-range-2 render ---
#table(
  columns: (1fr, 1fr),
  lines(6),
  [
    #text(lang: "ar", font: ("Libertinus Serif", "Noto Sans Arabic"))[مجرد نص مؤقت لأغراض العرض التوضيحي. ]
    #text(lang: "ar")[سلام]
  ],
)

--- issue-5276-shaping-consecutive-ltr-with-lang render ---
#let a = text(lang: "ar")[\u{645}]
#a#a
