#let c = counter("test")
#c.update(0)
#context {
  let n = c.get()
  c.step()
  pagebreak()
  [RESULT_START:#n:RESULT_END]
}