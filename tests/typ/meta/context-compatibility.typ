// Test compatibility with the pre-context way of things.
// Ref: false

---
#let s = state("x", 0)
#let compute(expr) = [
  #s.update(x =>
    eval(expr.replace("x", str(x)))
  )
  New value is #s.display().
]

#locate(loc => {
  let elem = query(<here>, loc).first()
  test(s.at(elem.location()), 13)
})

#compute("10") \
#compute("x + 3") \
*Here.* <here> \
#compute("x * 2") \
#compute("x - 5")

---
#style(styles => measure([it], styles).width < 20pt)

---
#counter(heading).update(10)
#counter(heading).display(n => test(n, 10))
