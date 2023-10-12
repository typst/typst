// Test that gradient fills on text.
// The solid bar gradients are used to make sure that all transforms are
// correct: if you can see the text through the bar, then the gradient is
// misaligned to its reference container.
// Ref: true

---
// Ref: false
// Make sure they don't work when `relative: "self"`.

// Hint: 17-61 make sure to set `relative: auto` on your text fill
// Error: 17-61 gradients on text must be relative to the parent
#set text(fill: gradient.linear(red, blue, relative: "self"))

---
// Test that gradient fills on text work for globally defined gradients.

#set page(width: 200pt, height: auto, margin: 10pt, background: {
  rect(width: 100%, height: 30pt, fill: gradient.linear(red, blue))
})
#set par(justify: true)
#set text(fill: gradient.linear(red, blue))
#lorem(30)

---
// Sanity check that the direction works on text.

#set page(width: 200pt, height: auto, margin: 10pt, background: {
  rect(height: 100%, width: 30pt, fill: gradient.linear(dir: btt, red, blue))
})
#set par(justify: true)
#set text(fill: gradient.linear(dir: btt, red, blue))
#lorem(30)

---
// Test that gradient fills on text work for locally defined gradients.

#set page(width: auto, height: auto, margin: 10pt)
#show box: set text(fill: gradient.linear(..color.map.rainbow))

Hello, #box[World]!

---
// Test that gradients fills on text work with transforms.

#set page(width: auto, height: auto, margin: 10pt)
#show box: set text(fill: gradient.linear(..color.map.rainbow))

#rotate(45deg, box[World])
