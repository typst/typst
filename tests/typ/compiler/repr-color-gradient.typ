// Test representation of values in the document.

---
// Colors
#set page(width: 400pt)
#set text(0.8em)
#blue \
#color.linear-rgb(blue) \
#oklab(blue) \
#cmyk(blue) \
#color.hsl(blue) \
#color.hsv(blue) \
#luma(blue)

---
// Gradients
#set page(width: 400pt)
#set text(0.8em)
#gradient.linear(blue, red) \
#gradient.linear(blue, red, dir: ttb) \
#gradient.linear(blue, red, angle: 45deg, relative: "self") \
#gradient.linear(blue, red, angle: 45deg, space: rgb)
