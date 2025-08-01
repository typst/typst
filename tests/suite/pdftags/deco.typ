--- deco-tags-underline pdftags ---
#show: underline.with(stroke: red)

// The content in this paragraph is grouped into one span tag with the
// corresponding text attributes.
red underlined text
red underlined text

red underlined text

--- deco-tags-different-color pdftags ---
#show: underline.with(stroke: red)
red underlined text
#show: underline.with(stroke: blue)
blue underlined text

--- deco-tags-different-stroke-thickness pdftags ---
#show: underline.with(stroke: 2pt)
red underlined text
#show: underline.with(stroke: 1pt)
blue underlined text

--- deco-tags-different-type pdftags ---
#underline[underlined]\
#overline[overlined]\
#strike[striked]\
