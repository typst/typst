// Test OpenType features.

--- text-kerning ---
// Test turning kerning off.
#text(kerning: true)[Tq] \
#text(kerning: false)[Tq]

--- text-alternates-and-stylistic-sets ---
// Test alternates and stylistic sets.
#set text(font: "IBM Plex Serif")
a vs #text(alternates: true)[a] \
ß vs #text(stylistic-set: 5)[ß] \
10 years ago vs #text(stylistic-set: (1, 2, 3))[10 years ago]

--- text-ligatures ---
// Test text turning off (standard) ligatures of the font.
#text(ligatures: false)[fi Qu] vs fi Qu \
// Test text turning on historical ligatures of the font.
abstract vs #text(historical-ligatures: true)[abstract] \
// Test text turning on discretionary ligatures of the font.
waltz vs #text(discretionary-ligatures: true)[waltz]

--- text-number-type ---
// Test number type.
#set text(number-type: "old-style")
0123456789 \
#text(number-type: auto)[0123456789]

--- text-number-width ---
// Test number width.
#text(number-width: "proportional")[0123456789] \
#text(number-width: "tabular")[3456789123] \
#text(number-width: "tabular")[0123456789]

--- text-slashed-zero-and-fractions ---
// Test extra number stuff.
#set text(font: "IBM Plex Serif")
0 vs. #text(slashed-zero: true)[0] \
1/2 vs. #text(fractions: true)[1/2]

--- text-features ---
// Test raw features array or dictionary.
#text(features: ("smcp",))[Smcp] \
fi vs. #text(features: (liga: 0))[No fi]

--- text-stylistic-set-bad-type ---
// Error: 26-31 expected none, integer, or array, found boolean
#set text(stylistic-set: false)

--- text-stylistic-set-out-of-bounds ---
// Error: 26-28 stylistic set must be between 1 and 20
#set text(stylistic-set: 25)

--- text-number-type-bad ---
// Error: 24-25 expected "lining", "old-style", or auto, found integer
#set text(number-type: 2)

--- text-features-bad ---
// Error: 21-26 expected array or dictionary, found boolean
#set text(features: false)

--- text-features-bad-nested-type ---
// Error: 21-35 expected string, found boolean
#set text(features: ("tag", false))

--- text-tracking-negative ---
// Test tracking.
#set text(tracking: -0.01em)
I saw Zoe yӛsterday, on the tram.

--- text-tracking-changed-temporarily ---
// Test tracking for only part of paragraph.
I'm in#text(tracking: 0.15em + 1.5pt)[ spaace]!

--- text-tracking-mark-placement ---
// Test that tracking doesn't disrupt mark placement.
#set text(font: ("PT Sans", "Noto Serif Hebrew"))
#set text(tracking: 0.3em)
טֶקסט

--- text-tracking-arabic ---
// Test tracking in arabic text (makes no sense whatsoever)
#set text(tracking: 0.3em)
النص

--- text-spacing ---
// Test word spacing.
#set text(spacing: 1em)
My text has spaces.

--- text-spacing-relative ---
// Test word spacing relative to the font's space width.
#set text(spacing: 50% + 1pt)
This is tight.
