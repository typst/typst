// Test layout transformations
---

// Test that rotation impact layout.
#set page(width: 200pt)
#set rotate(layout: true)
#let one(angle) = box(fill:blue, rotate([Test Text], angle * 1deg))

#for angle in range(0, 360, step: 15) {
  one(angle)
}

---

// Test that scaling impact layout.
#set page(width: 200pt)
#set text(size: 32pt)
#let scaled(body) = box(scale(x: 20%, y: 40%, body))

#set scale(layout: false)
Hello #scaled[World]!


#set scale(layout: true)
Hello #scaled[World]!

---

// Test that translation impact layout.
#set page(width: 200pt)
#set text(size: 32pt)
#let moved(body) = box(move(dx: -10pt, dy: 5pt, body))

#set move(layout: false)
Hello #moved[World]!


#set move(layout: true)
Hello #moved[World]!
