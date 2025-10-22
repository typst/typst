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

--- measure-counter-width ---
// Measure a counter. Tests that the introspector-assisted location assignment
// is able to take `here()` from the context into account to find the closest
// matching element instead of any single one. Crucially, we need to reuse
// the same `context c.display()` to get the same span, hence `it`.
#let f(it) = context [
  Is #measure(it).width wide: #it \
]

#let c = counter("c")
#let it = context c.display()

#c.update(10000)
#f(it)
#c.update(100)
#f(it)
#c.update(1)
#f(it)

--- measure-citation-in-flow ---
// Try measuring a citation that appears inline with other stuff. The
// introspection-assisted location assignment will ensure that the citation
// in the measurement is matched up with the real one.
#context {
  let it = [@netwok]
  let size = measure(it)
  place(line(length: size.width))
  v(1mm)
  it + [ is cited]
}

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- measure-citation-in-flow-different-span ---
// When the citation has a different span, it stops working.
#context {
  // Error: 22-29 cannot format citation in isolation
  // Hint: 22-29 check whether this citation is measured without being inserted into the document
  let size = measure[@netwok]
  place(line(length: size.width))
  v(1mm)
  [@netwok is cited]
}

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- measure-citation-deeply-nested ---
// Nested the citation deeply to test that introspector-assisted measurement
// is able to deal with memoization boundaries.
#context {
  let it = box(pad(x: 5pt, grid(stack[@netwok])))
  [#measure(it).width]
  it
}

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- measure-counter-multiple-times ---
// When the thing we measure appears multiple times, we measure as if it was
// the first one.
#context {
  let c = counter("c")
  let u(n) = c.update(n)
  let it = context c.get().first() * h(1pt)
  let size = measure(it)
  table(columns: 5, u(17), it, u(1), it, u(5))
  [#size.width] // 17pt
}

--- issue-5180-measure-inline-math-bounds ---
#context {
  let height = measure(text(top-edge: "bounds", $x$)).height
  assert(height > 4pt)
  assert(height < 5pt)
}

--- measure-html html ---
#context {
  let (width, height) = measure(image("/assets/images/monkey.svg"))
  test(width, 36pt)
  test(height, 36pt)
}
