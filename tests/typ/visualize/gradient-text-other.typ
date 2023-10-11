// Test text gradients with radial and conic gradients.

---
#set page(width: 200pt, height: auto, margin: 10pt)
#set par(justify: true)
#set text(fill: gradient.radial(red, blue))
#lorem(30)


---
#set page(width: 200pt, height: auto, margin: 10pt)
#set par(justify: true)
#set text(fill: gradient.conic(red, blue, angle: 45deg))
#lorem(30)
