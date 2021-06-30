// Test configuring font properties.

---
// Set same font size in three different ways.
#font(size: 22pt)[A]
#font(size: 200%)[A]
#font(size: 16.5pt + 50%)[A]

// Do nothing.
#font[Normal]

// Set style (is available).
#font(style: italic)[Italic]

// Set weight (is available).
#font(weight: bold)[Bold]

// Set stretch (not available, matching closest).
#font(stretch: 50%)[Condensed]

// Set family.
#font(family: "PT Sans")[Sans serif]

// Emoji.
Emoji: 🐪, 🌋, 🏞

// Math.
#font(family: "Latin Modern Math")[
    ∫ 𝛼 + 3𝛽 d𝑡
]

// Colors.
#font(color: eastern)[This is #font(color: rgb("FA644B"))[way more] colorful.]

---
// Test top and bottom edge.

#page!(width: 170pt)
#let try(top, bottom) = rect(fill: conifer)[
    #font!(top-edge: top, bottom-edge: bottom)
    `From `#top` to `#bottom
]

#try(ascender, descender)
#try(ascender, baseline)
#try(cap-height, baseline)
#try(x-height, baseline)

---
// Test class definitions.
#font!(sans-serif: "PT Sans")
#font(family: sans-serif)[Sans-serif.] \
#font(family: monospace)[Monospace.] \
#font(family: monospace, monospace: ("Nope", "Latin Modern Math"))[Math.]

---
// Ref: false

// Error: 7-12 unexpected argument
#font(false)[]

// Error: 3:14-3:18 expected font style, found font weight
// Error: 2:28-2:34 expected font weight, found string
// Error: 1:43-1:44 expected string or array of strings, found integer
#font(style: bold, weight: "thin", serif: 0)[]

// Warning: 15-19 should be between 100 and 900
#font(weight: 2700)[]

// Warning: 16-21 should be between 50% and 200%
#font(stretch: 1000%)[]

// Error: 7-27 unexpected argument
#font(something: "invalid")[]
