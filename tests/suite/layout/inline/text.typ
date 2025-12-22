// Test OpenType features.

--- text-kerning paged ---
// Test turning kerning off.
#text(kerning: true)[Tq] \
#text(kerning: false)[Tq]

--- text-alternates-and-stylistic-sets paged ---
// Test alternates and stylistic sets.
#set text(font: "IBM Plex Serif")
a vs #text(alternates: true)[a] \
ß vs #text(stylistic-set: 5)[ß] \
10 years ago vs #text(stylistic-set: (1, 2, 3))[10 years ago]

--- text-alternates-int paged ---
// Test selecting between multiple alternates.
#set text(font: "Libertinus Serif")
#text(alternates: false, [ß]) vs #text(alternates: true, [ß]) vs #text(alternates: 2, [ß])

--- text-ligatures paged ---
// Test text turning off (standard) ligatures of the font.
#text(ligatures: false)[fi Qu] vs fi Qu \
// Test text turning on historical ligatures of the font.
abstract vs #text(historical-ligatures: true)[abstract] \
// Test text turning on discretionary ligatures of the font.
waltz vs #text(discretionary-ligatures: true)[waltz]

--- text-number-type paged ---
// Test number type.
#set text(number-type: "old-style")
0123456789 \
#text(number-type: auto)[0123456789]

--- text-number-width paged ---
// Test number width.
#text(number-width: "proportional")[0123456789] \
#text(number-width: "tabular")[3456789123] \
#text(number-width: "tabular")[0123456789]

--- text-slashed-zero-and-fractions paged ---
// Test extra number stuff.
#set text(font: "IBM Plex Serif")
0 vs. #text(slashed-zero: true)[0] \
1/2 vs. #text(fractions: true)[1/2]

--- text-features paged ---
// Test raw features array or dictionary.
#text(features: ("smcp",))[Smcp] \
fi vs. #text(features: (liga: 0))[No fi]

--- text-stylistic-set-bad-type paged ---
// Error: 26-31 expected none, integer, or array, found boolean
#set text(stylistic-set: false)

--- text-stylistic-set-out-of-bounds paged ---
// Error: 26-28 stylistic set must be between 1 and 20
#set text(stylistic-set: 25)

--- text-number-type-bad paged ---
// Error: 24-25 expected "lining", "old-style", or auto, found integer
#set text(number-type: 2)

--- text-features-bad paged ---
// Error: 21-26 expected array or dictionary, found boolean
#set text(features: false)

--- text-features-non-ascii paged ---
// Error: 21-30 feature tag may contain only printable ASCII characters
// Hint: 21-30 found invalid cluster `"ƒ"`
// Hint: 21-30 occurred in tag at index 0 (`"ƒeat"`)
#set text(features: ("ƒeat",))

--- text-features-bad-padding paged ---
// Error: 21-30 spaces may only appear as padding following a feature tag
// Hint: 21-30 occurred in tag at index 0 (`" tag"`)
#set text(features: (" tag",))

--- text-features-empty-array paged ---
// Error: 21-26 feature tag must be one to four characters in length
// Hint: 21-26 found 0 characters
// Hint: 21-26 occurred in tag at index 0 (`""`)
#set text(features: ("",))

--- text-features-overlong-dict paged ---
// Error: 21-41 feature tag must be one to four characters in length
// Hint: 21-41 found 15 characters
// Hint: 21-41 occurred in tag at index 0 (`"verylongfeature"`)
#set text(features: (verylongfeature: 0))

--- text-features-array-kv paged ---
// Error: 21-32 feature tag must be one to four characters in length
// Hint: 21-32 found 6 characters
// Hint: 21-32 occurred in tag at index 0 (`"feat=2"`)
// Hint: 21-32 to set features with custom values, consider supplying a dictionary
#set text(features: ("feat=2",))

--- text-features-bad-nested-type paged ---
// Error: 21-35 expected string, found boolean
// Hint: 21-35 occurred in tag at index 1 (`false`)
// Hint: 21-35 to set features with custom values, consider supplying a dictionary
#set text(features: ("tag", false))

--- text-tracking-negative paged ---
// Test tracking.
#set text(tracking: -0.01em)
I saw Zoe yӛsterday, on the tram.

--- text-tracking-changed-temporarily paged ---
// Test tracking for only part of paragraph.
I'm in#text(tracking: 0.15em + 1.5pt)[ spaace]!

--- text-tracking-mark-placement paged ---
// Test that tracking doesn't disrupt mark placement.
#set text(font: ("PT Sans", "Noto Serif Hebrew"))
#set text(tracking: 0.3em)
טֶקסט

--- text-tracking-arabic paged ---
// Test tracking in arabic text (makes no sense whatsoever)
#set text(tracking: 0.3em, font: "Noto Sans Arabic")
النص

--- text-spacing paged ---
// Test word spacing.
#set text(spacing: 1em)
My text has spaces.

--- text-spacing-relative paged ---
// Test word spacing relative to the font's space width.
#set text(spacing: 50% + 1pt)
This is tight.
