// Test configuring font properties.

[font: "PT Sans", 10pt]

// Set same font size in three different ways.
[font: 20pt][A]
[font: 200%][A]
[font: 15pt + 50%][A]

// Do nothing.
[font][Normal]

// Set style (is available).
[font: style=italic][Italic]

// Set weight (is available).
[font: weight=bold][Bold]

// Set stretch (not available, matching closest).
[font: stretch=ultra-condensed][Condensed]
