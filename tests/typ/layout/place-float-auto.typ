// Test floating placement.

---
#set page(height: 140pt)
#set place(clearance: 5pt)
#lorem(6)
#place(auto, float: true, rect[A])
#place(auto, float: true, rect[B])
#place(auto, float: true, rect[C])
#place(auto, float: true, rect[D])

---
// Error: 2-20 automatic positioning is only available for floating placement
// Hint: 2-20 you can enable floating placement with `place(float: true, ..)`
#place(auto)[Hello]

---
// Error: 2-45 floating placement must be `auto`, `top`, or `bottom`
#place(center + horizon, float: true)[Hello]
