// Test configuring font properties.

// Set same font size in three different ways.
#font(22pt)[A]
#font(200%)[A]
#font(16.5pt + 50%)[A]

// Do nothing.
#font[Normal]

// Set style (is available).
#font(style: italic)[Italic]

// Set weight (is available).
#font(weight: bold)[Bold]

// Set stretch (not available, matching closest).
#font(stretch: ultra-condensed)[Condensed]

// Set family.
#font("PT Sans")[Sans serif]

// Emoji.
Emoji: 🐪, 🌋, 🏞

// Math.
#font("Latin Modern Math")[
    ∫ 𝛼 + 3𝛽 d𝑡
]

---
// Ref: false

// Error: 7-12 unexpected argument
#font(false)

// Error: 3:14-3:18 expected font style, found font weight
// Error: 2:28-2:34 expected font weight, found string
// Error: 1:43-1:44 expected font family or array of font families, found integer
#font(style: bold, weight: "thin", serif: 0)

// Warning: 15-19 should be between 100 and 900
#font(weight: 2700)

// Error: 7-27 unexpected argument
#font(something: "invalid")
