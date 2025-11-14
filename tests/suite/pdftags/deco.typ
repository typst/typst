--- deco-tags-underline pdftags pdfstandard(ua-1) ---
#show: underline.with(stroke: red)

// The content in this paragraph is grouped into one span tag with the
// corresponding text attributes.
red underlined text
red underlined text

red underlined text

--- deco-tags-different-color pdftags pdfstandard(ua-1) ---
#show: underline.with(stroke: red)
red underlined text
#show: underline.with(stroke: blue)
blue underlined text

--- deco-tags-different-stroke-thickness pdftags pdfstandard(ua-1) ---
#show: underline.with(stroke: 2pt)
thick underlined
#show: underline.with(stroke: 1pt)
thin underlined

--- deco-tags-different-type pdftags pdfstandard(ua-1) ---
#underline[underlined]\
#overline[overlined]\
#strike[striked]\

--- deco-tags-multiple-styles pdftags pdfstandard(ua-1) ---
#show: underline
// Error: 2-16 PDF/UA-1 error: cannot combine underline, overline, or strike
#show: overline
text with a bunch of lines

--- deco-tags-highlight-basic pdftags pdfstandard(ua-1) ---
A #highlight[highlighted] alksjdflk asdjlkfj alskdj word.

--- deco-tags-subscript-basic pdftags pdfstandard(ua-1) ---
CO#sub[2] emissions.
A2#sub[hex]

--- deco-tags-superscript-basic pdftags pdfstandard(ua-1) ---
CI#super[-] has a negative charge.

--- deco-tags-script-custom-baseline pdftags pdfstandard(ua-1) ---
// NOTE: the baseline shift values attribute is inverted.
#set sub(baseline: 2.5pt)
#set super(baseline: -9.5pt)
#sub[sub]
#super[super]

--- deco-tags-emph-basic pdftags pdfstandard(ua-1) ---
Cats are _cute_ animals.

--- deco-tags-strong-basic pdftags pdfstandard(ua-1) ---
This *HERE* is important!

--- deco-tags-strong-and-em pdftags pdfstandard(ua-1) ---
_*strong and emph*_

--- deco-tags-strong-em-and-more-attrs pdftags pdfstandard(ua-1) ---
#underline(stroke: green)[_*strong and emph*_]
