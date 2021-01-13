// Test configuring font properties.

[font "PT Sans", 10pt]

// Set same font size in three different ways.
[font 20pt][A]
[font 200%][A]
[font 15pt + 50%][A]

// Do nothing.
[font][Normal]

// Set style (is available).
[font style: italic][Italic]

// Set weight (is available).
[font weight: bold][Bold]

// Set stretch (not available, matching closest).
[font stretch: ultra-condensed][Condensed]

---
// Test font fallback.

// Source Sans Pro + Segoe UI Emoji.
Emoji: 🏀

// CMU Serif + Noto Emoji.
[font "CMU Serif", "Noto Emoji"][
    Emoji: 🏀
]

// Class definitions.
[font serif: ("CMU Serif", "Latin Modern Math", "Noto Emoji")]
[font serif][
    Math: ∫ α + β ➗ 3
]

// Class definition reused.
[font sans-serif: "Noto Emoji"]
[font sans-serif: ("Archivo", sans-serif)]
New sans-serif. 🚀

---
// Test error cases.
//
// ref: false
// error: 3:7-3:12 unexpected argument
// error: 6:14-6:18 expected font style, found font weight
// error: 6:28-6:34 expected font weight, found string
// error: 6:43-6:44 expected font family or array of font families, found integer
// warning: 9:15-9:19 must be between 100 and 900
// error: 12:7-12:27 unexpected argument

// Not one of the valid things for positional arguments.
[font false]

// Wrong types.
[font style: bold, weight: "thin", serif: 0]

// Weight out of range.
[font weight: 2700]

// Non-existing argument.
[font something: "invalid"]
