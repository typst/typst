// Test locate me.

---
#set page(height: 60pt)
#let pin = locate(me => box({
  let c(length) = str(int(length / 1pt ) )
  square(size: 1.5pt, fill: blue)
  h(0.15em)
  text(0.5em)[{me.page}, #c(me.x), #c(me.y)]
}))

#place(rotate(origin: top + left, 25deg, move(dx: 40pt, pin)))

#pin
#h(10pt)
#box(pin) \
#pin

#place(bottom + right, pin)

#pagebreak()
#align(center + horizon, pin + [\ ] + pin)
