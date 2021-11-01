// Test configuring font properties.

---
// Set same font size in three different ways.
#font(20pt)[A]
#font(200%)[A]
#font(size: 15pt + 50%)[A]

// Do nothing.
#font()[Normal]

// Set style (is available).
#font(style: "italic")[Italic]

// Set weight (is available).
#font(weight: "bold")[Bold]

// Set stretch (not available, matching closest).
#font(stretch: 50%)[Condensed]

// Set family.
#font(family: serif)[Serif]

// Emoji.
Emoji: 🐪, 🌋, 🏞

// Math.
#font("Latin Modern Math")[∫ 𝛼 + 3𝛽 d𝑡]

// Colors.
[
  #font(fill: eastern)
  This is #font(rgb("FA644B"))[way more] colorful.
]

// Disable font fallback beyond the user-specified list.
// Without disabling, Latin Modern Math would come to the rescue.
#font("PT Sans", "Twitter Color Emoji", fallback: false)
2π = 𝛼 + 𝛽. ✅

---
// Test class definitions.
#font(sans-serif: "PT Sans")
#font(family: sans-serif)[Sans-serif.] \
#font(monospace)[Monospace.] \
#font(monospace, monospace: ("Nope", "Latin Modern Math"))[Math.]

---
// Test top and bottom edge.

#page(width: 150pt)
#font(size: 8pt)

#let try(top, bottom) = rect(fill: conifer)[
  #font(monospace, top-edge: top, bottom-edge: bottom)
  From #top to #bottom
]

#try("ascender", "descender")
#try("ascender", "baseline")
#try("cap-height", "baseline")
#try("x-height", "baseline")
#try(4pt, -2pt)
#try(1pt + 27%, -18%)

---
// Error: 7-12 unexpected argument
#font(false)

---
// Error: 14-20 expected "normal", "italic" or "oblique"
#font(style: "bold", weight: "thin")

---
// Error: 17-19 expected linear or string, found array
#font(top-edge: ())

---
// Error: 17-19 unknown font metric
#font(top-edge: "")

---
// Error: 14-15 expected string or array of strings, found integer
#font(serif: 0)

---
// Error: 19-23 unexpected argument
#font(size: 10pt, 12pt)

---
// Error: 28-35 unexpected argument
#font(family: "Helvetica", "Arial")

---
// Error: 7-27 unexpected argument
#font(something: "invalid")
