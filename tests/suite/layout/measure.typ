--- measure ---
// Test `measure`.
#let f(lo, hi) = context {
  let h = measure[Hello].height
  assert(h > lo)
  assert(h < hi)
}
#text(10pt, f(6pt, 8pt))
#text(20pt, f(13pt, 14pt))

--- measure-given-area ---
// Test `measure` given an area.
#let text = lorem(100)

#context {
  let d1 = measure(text)
  assert(d1.width > 2000pt)
  assert(d1.height < 10pt)
  let d2 = measure(width: 400pt, height: auto, text)
  assert(d2.width < 400pt)
  assert(d2.height > 50pt)
}
