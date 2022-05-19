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

// Set family.
#text(family: "IBM Plex Serif")[Serif]

// Emoji.
Emoji: 🐪, 🌋, 🏞

// Colors.
[
  #set text(fill: eastern)
  This is #text(rgb("FA644B"))[way more] colorful.
]

// Disable font fallback beyond the user-specified list.
// Without disabling, NewComputerModernMath would come to the rescue.
#set text("PT Sans", "Twitter Color Emoji", fallback: false)
2π = 𝛼 + 𝛽. ✅

---
// Test string body.
#text("Text") \
#text(red, "Text") \
#text("Ubuntu", blue, "Text") \
#text([Text], teal, "IBM Plex Serif") \
#text(forest, "Latin Modern Roman", [Text]) \

---
// Error: 11-16 unexpected argument
#set text(false)

---
// Error: 18-24 expected "normal", "italic" or "oblique"
#set text(style: "bold", weight: "thin")

---
// Error: 23-27 unexpected argument
#set text(size: 10pt, 12pt)

---
// Error: 32-39 unexpected argument
#set text(family: "Helvetica", "Arial")

---
// Error: 11-31 unexpected argument
#set text(something: "invalid")
