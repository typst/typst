// Test configuring font properties.

--- text-font-properties ---
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

// Transparency.
#block(fill: green)[
  #set text(fill: rgb("FF000080"))
  This text is transparent.
]

// Disable font fallback beyond the user-specified list.
// Without disabling, New Computer Modern Math would come to the rescue.
#set text(font: ("PT Sans", "Twitter Color Emoji"), fallback: false)
2π = 𝛼 + 𝛽. ✅

--- text-call-body ---
// Test string body.
#text("Text") \
#text(red, "Text") \
#text(font: "Ubuntu", blue, "Text") \
#text([Text], teal, font: "IBM Plex Serif") \
#text(forest, font: "New Computer Modern", [Text]) \

--- text-bad-argument ---
// Error: 11-16 unexpected argument
#set text(false)

--- text-style-bad ---
// Error: 18-24 expected "normal", "italic", or "oblique"
#set text(style: "bold", weight: "thin")

--- text-bad-extra-argument ---
// Error: 23-27 unexpected argument
#set text(size: 10pt, 12pt)

--- text-bad-named-argument ---
// Error: 11-31 unexpected argument: something
#set text(something: "invalid")

--- text-unknown-font-family-warning ---
#text(font: "libertinus serif")[I exist,]
// Warning: 13-26 unknown font family: nonexistent
#text(font: "nonexistent")[but]
// Warning: 17-35 unknown font family: also-nonexistent
#set text(font: "also-nonexistent")
I
// Warning: 23-55 unknown font family: list-of
// Warning: 23-55 unknown font family: nonexistent-fonts
#let var = text(font: ("list-of", "nonexistent-fonts"))[don't]
#var

--- text-font-linux-libertine ---
// Warning: 17-34 Typst's default font has changed from Linux Libertine to its successor Libertinus Serif
// Hint: 17-34 please set the font to `"Libertinus Serif"` instead
#set text(font: "Linux Libertine")

--- issue-5499-text-fill-in-clip-block ---

#let t = tiling(
  size: (30pt, 30pt),
  relative: "parent",
  square(
    size: 30pt,
    fill: gradient
      .conic(..color.map.rainbow),
  )
)

#block(clip: false, height: 2em, {
  text(fill: blue, "Hello")
  [ ]
  text(fill: blue.darken(20%).transparentize(50%), "Hello")
  [ ]
  text(fill: gradient.linear(..color.map.rainbow), "Hello")
  [ ]
  text(fill: t, "Hello")
})
#block(clip: true, height: 2em, {
  text(fill: blue, "Hello")
  [ ]
  text(fill: blue.darken(20%).transparentize(50%), "Hello")
  [ ]
  text(fill: gradient.linear(..color.map.rainbow), "Hello")
  [ ]
  text(fill: t, "Hello")
})
