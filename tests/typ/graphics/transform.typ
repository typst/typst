// Test transformations.

---
// Test creating the TeX and XeTeX logos.
#let size = 11pt
#let tex = [{
  [T]
  h(-0.14 * size)
  move(dy: 0.22 * size)[E]
  h(-0.12 * size)
  [X]
}]

#let xetex = {
  [X]
  h(-0.14 * size)
  scale(x: -100%, move(dy: 0.26 * size)[E])
  h(-0.14 * size)
  [T]
  h(-0.14 * size)
  move(dy: 0.26 * size)[E]
  h(-0.12 * size)
  [X]
}

#set text("Latin Modern Roman", size)
Neither #tex, \
nor #xetex!

---
// Test combination of scaling and rotation.
#set page(height: 80pt)
#align(center + horizon,
  rotate(20deg, scale(70%, image("/res/tiger.jpg")))
)

---
// Test setting rotation origin.
#rotate(10deg, origin: top + left,
  image("/res/tiger.jpg", width: 50%)
)

---
// Test setting scaling origin.
#let r = rect(width: 100pt, height: 10pt, fill: forest)
#set page(height: 65pt)
#scale(r, x: 50%, y: 200%, origin: left + top)
#scale(r, x: 50%, origin: center)
#scale(r, x: 50%, y: 200%, origin: right + bottom)
