// SKIP

// Calls `f` with the current layout iteration and displays the return
// value of `f`.
#let switch(f) = context {
  // The `here()` trick is just to produce a counter unique to this context
  // block.
  let c = counter(metadata.where(value: here()))
  let i = c.final().first()
  let n = i + 1
  c.update(if n < 5 { i + 1 } else { i })
  context f(n)
}
