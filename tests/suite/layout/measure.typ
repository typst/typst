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

#context measure(text)
#context measure(width: 400pt, height: auto, text)
