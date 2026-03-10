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
