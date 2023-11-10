// Test localization-related figure features.

---
// Test French
#set text(lang: "fr")
#figure(
  circle(),
  caption: [Un cercle.],
)

---
// Test Chinese
#set text(lang: "zh")
#figure(
  rect(),
  caption: [一个矩形],
)
