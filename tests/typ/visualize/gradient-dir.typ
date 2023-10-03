// Test gradients with direction.

---
#set page(width: 900pt)
#for i in range(0, 360, step: 15){
  box(
    height: 100pt,
    width: 100pt,
    fill: gradient.linear(angle: i * 1deg, (red, 0%), (blue, 100%)),
    align(center + horizon)[Angle: #i degrees],
  )
  h(30pt)
}
