--- smallcaps ---
// Test smallcaps.
#smallcaps[Smallcaps]

--- smallcaps-show-rule ---
// There is no dedicated smallcaps font in typst-dev-assets, so we just use some
// other font to test this show rule.
#show smallcaps: set text(font: "PT Sans")
#smallcaps[Smallcaps]

#show smallcaps: set text(fill: red)
#smallcaps[Smallcaps]

--- smallcaps-arguments ---
#smallcaps(lowercase: false, uppercase: false)[Test 012] \
#smallcaps(lowercase: false, uppercase: true)[Test 012] \
#smallcaps(lowercase: true, uppercase: false)[Test 012] \
#smallcaps(lowercase: true, uppercase: true)[Test 012]
