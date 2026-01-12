--- smallcaps paged ---
// Test smallcaps.
#smallcaps[Smallcaps]

--- smallcaps-show-rule paged ---
// There is no dedicated smallcaps font in typst-dev-assets, so we just use some
// other font to test this show rule.
#show smallcaps: set text(font: "PT Sans")
#smallcaps[Smallcaps]

#show smallcaps: set text(fill: red)
#smallcaps[Smallcaps]

--- smallcaps-all paged html ---
#smallcaps(all: false)[Test 012] \
#smallcaps(all: true)[Test 012]

--- smallcaps-typographic paged ---
// Test typographic vs synthesized smallcaps.
// Using a font that doesn't have smcp feature, synthesis should kick in.
#set text(size: 20pt)

// Default is typographic: true, but synthesis will be used as fallback
// if the font doesn't have smcp.
#smallcaps[Hello World] \
#smallcaps(typographic: false)[Hello World]

--- smallcaps-typographic-with-all paged ---
// Test typographic combined with all parameter.
#set text(size: 20pt)

#smallcaps(all: true, typographic: false)[HELLO world] \
#smallcaps(all: false)[HELLO world]

--- smallcaps-multibyte paged ---
// Test smallcaps with multibyte characters that expand when uppercased.
// German ß becomes SS when uppercased.
#set text(size: 16pt)

#smallcaps[Straße München] \
#smallcaps(typographic: false)[Straße München] \
#smallcaps[Größe Süß] \
#smallcaps(all: true)[GRÖßE SÜß]

--- smallcaps-mixed-case paged ---
// Test smallcaps with mixed case text.
#set text(size: 16pt)

#smallcaps[Hello WORLD 123] \
#smallcaps(all: false)[Hello WORLD 123] \
#smallcaps(all: true)[Hello WORLD 123]

--- smallcaps-non-letter paged ---
// Test that non-letter characters are preserved.
#set text(size: 16pt)

#smallcaps[test\@example.com] \
#smallcaps[Typst is #super[cool]!] \
#smallcaps(all: true)[Test 123 & More!]
