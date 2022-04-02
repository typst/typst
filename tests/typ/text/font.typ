// Test configuring font properties.

---
// Set same font size in three different ways.
#text(20pt)[A]
#text(200%)[A]
#text(size: 15pt + 50%)[A]

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

// Math.
#text("Latin Modern Math")[∫ 𝛼 + 3𝛽 d𝑡]

// Colors.
[
  #set text(fill: eastern)
  This is #text(rgb("FA644B"))[way more] colorful.
]

// Disable font fallback beyond the user-specified list.
// Without disabling, Latin Modern Math would come to the rescue.
#set text("PT Sans", "Twitter Color Emoji", fallback: false)
2π = 𝛼 + 𝛽. ✅

---
// Test top and bottom edge.
#set page(width: 150pt)
#set text(size: 8pt)

#let try(top, bottom) = rect(fill: conifer)[
  #set text("IBM Plex Mono", top-edge: top, bottom-edge: bottom)
  From #top to #bottom
]

#try("ascender", "descender")
#try("ascender", "baseline")
#try("cap-height", "baseline")
#try("x-height", "baseline")
#try(4pt, -2pt)
#try(1pt + 27%, -18%)

---
// Error: 11-16 unexpected argument
#set text(false)

---
// Error: 18-24 expected "normal", "italic" or "oblique"
#set text(style: "bold", weight: "thin")

---
// Error: 21-23 expected linear or string, found array
#set text(top-edge: ())

---
// Error: 21-23 unknown font metric
#set text(top-edge: "")

---
// Error: 23-27 unexpected argument
#set text(size: 10pt, 12pt)

---
// Error: 32-39 unexpected argument
#set text(family: "Helvetica", "Arial")

---
// Error: 11-31 unexpected argument
#set text(something: "invalid")
