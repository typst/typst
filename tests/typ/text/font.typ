// Test configuring font properties.

---
// Set same font size in three different ways.
#text(20pt)[A]
#text(2em)[A]
#text(size: 15pt + 0.5em)[A]

// Do nothing.
#text()[Normal]

// Set style (is available).
#text(style: "italic")[Italic]

// Set weight (is available).
#text(weight: "bold")[Bold]

// Set stretch (not available, matching closest).
#text(stretch: 50%)[Condensed]

// Set font family.
#text(font: "IBM Plex Serif")[Serif]

// Emoji.
Emoji: 🐪, 🌋, 🏞

// Colors.
#[
  #set text(fill: eastern)
  This is #text(rgb("FA644B"))[way more] colorful.
]

// Disable font fallback beyond the user-specified list.
// Without disabling, New Computer Modern Math would come to the rescue.
#set text(font: ("PT Sans", "Twitter Color Emoji"), fallback: false)
2π = 𝛼 + 𝛽. ✅

---
// Test string body.
#text("Text") \
#text(red, "Text") \
#text(font: "Ubuntu", blue, "Text") \
#text([Text], teal, font: "IBM Plex Serif") \
#text(forest, font: "New Computer Modern", [Text]) \

---
// Error: 11-16 unexpected argument
#set text(false)

---
// Error: 18-24 expected "normal", "italic", or "oblique"
#set text(style: "bold", weight: "thin")

---
// Error: 23-27 unexpected argument
#set text(size: 10pt, 12pt)

---
// Error: 11-31 unexpected argument: something
#set text(something: "invalid")
