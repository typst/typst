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

---
// Test Russian
#set text(lang: "ru")

#figure(
    polygon.regular(size: 1cm, vertices: 8),
    caption: [Пятиугольник],
)

---
// Test Greek
#set text(lang: "gr")
#figure(
  circle(),
  caption: [Ένας κύκλος.],
)
