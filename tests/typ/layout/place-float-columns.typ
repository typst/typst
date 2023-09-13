// Test floats in columns.

---
#set page(height: 200pt, width: 300pt)
#show: columns.with(2)

= Introduction
#figure(
  placement: bottom,
  caption: [A glacier],
  image("/files/glacier.jpg", width: 50%),
)
#lorem(45)
#figure(
  placement: top,
  caption: [A rectangle],
  rect[Hello!],
)
#lorem(20)
