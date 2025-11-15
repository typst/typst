// Tests convergence warnings.

--- convergence-query ---
// Warning: document did not converge within five attempts
// Hint: see 5 additional warnings for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch
#show strong: none

= Real <real>

#context {
  // Warning: 15-29 number of heading elements did not stabilize
  // Hint: 15-29 the following numbers of elements were observed:\n- run 1: 0\n- run 2: 1\n- run 3: 2\n- run 4: 3\n- run 5: 4\n- final: 5
  let elems = query(heading)
  let count = elems.len()
  count * [= Fake <fake>]
}

#context {
  // This one converges.
  _ = query(<real>)
}

// Test alternative warning messages.
#context {
  // Warning: 7-37 number of matching heading elements did not stabilize
  // Hint: 7-37 the following numbers of elements were observed:\n- run 1: 0\n- run 2: 1\n- run 3: 2\n- run 4: 3\n- run 5: 4\n- final: 5
  _ = query(heading.where(level: 1))

  // Warning: 7-20 number of elements labelled `<fake>` did not stabilize
  // Hint: 7-20 the following numbers of elements were observed:\n- run 1: 0\n- run 2: 0\n- run 3: 1\n- run 4: 2\n- run 5: 3\n- final: 4
  _ = query(<fake>)

  // Warning: 7-48 number of elements matching `selector.or(<fake>, heading.where(level: 1))` did not stabilize
  // Hint: 7-48 the following numbers of elements were observed:\n- run 1: 0\n- run 2: 1\n- run 3: 2\n- run 4: 3\n- run 5: 4\n- final: 5
  _ = query(heading.where(level: 1).or(<fake>))
}

// This one has no hint since the number of matching elements is the same and
// it's difficult to provide a good hint for the concrete elements.
#switch(n => if n == 4 [*A*] else [*B*])
#context {
  // Warning: 7-20 query for strong elements did not stabilize
  _ = query(strong)
}

--- convergence-query-first-and-unique ---
// Warning: document did not converge within five attempts
// Hint: see 2 additional warnings for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch
#switch(n => if n == 4 [*A* <a>] else if n == 5 [_A_ <a>])

// Warning: 2-17 query for a unique element labelled `<a>` did not stabilize
// Warning: 2-17 query for the first element matching `location(..)` did not stabilize
// Hint: 2-17 the following numbers of elements were observed:\n- run 1: 0\n- run 2: 0\n- run 3: 0\n- run 4: 0\n- run 5: 1\n- final: 0
#link(<a>)[Link]

--- convergence-query-label ---
// Warning: document did not converge within five attempts
// Hint: see 2 additional warnings for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch
#set heading(numbering: "1.")
= A

// Warning: 25-28 value of `counter(heading)` did not converge
// Hint: 25-28 the following values were observed:\n- run 1: 0\n- run 2: 1\n- run 3: 1\n- run 4: 1\n- run 5: 1\n- final: 2
#switch(n => if n == 5 [= B <b>])

// Error: 1-3 label `<b>` does not exist in the document
// Warning: 1-3 query for a unique element labelled `<b>` did not stabilize
// Hint: 1-3 the following numbers of elements were observed:\n- run 1: 0\n- run 2: 0\n- run 3: 0\n- run 4: 0\n- run 5: 0\n- final: 1
@b

--- convergence-position ---
// Warning: document did not converge within five attempts
// Hint: see 1 additional warning for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch

// Hint: 29-38 heading was created here
#switch(n => v(n * 10pt) + [= Heading])

// Warning: 10-36 heading position did not stabilize
// Hint: 10-36 the following positions were observed:\n- run 1: page 1 at (0pt, 0pt)\n- run 2: page 1 at (10pt, 20pt)\n- run 3: page 1 at (10pt, 30pt)\n- run 4: page 1 at (10pt, 40pt)\n- run 5: page 1 at (10pt, 50pt)\n- final: page 1 at (10pt, 60pt)
#context locate(heading).position().y

--- convergence-page ---
// Warning: document did not converge within five attempts
// Hint: see 2 additional warnings for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch

// No "heading was created here" hint because the heading does not exist anymore
// in the end.
#switch(n => if n == 4 { pagebreak() + [= Heading] })

// Warning: 10-25 query for a unique heading element did not stabilize
// Hint: 10-25 the following numbers of elements were observed:\n- run 1: 0\n- run 2: 0\n- run 3: 0\n- run 4: 0\n- run 5: 1\n- final: 0
// Warning: 10-32 page number of the element did not stabilize
// Hint: 10-32 the following page numbers were observed:\n- run 1: page 1\n- run 2: page 1\n- run 3: page 1\n- run 4: page 1\n- run 5: page 2\n- final: page 1
#context locate(heading).page()

--- convergence-page-supplement ---
// Warning: document did not converge within five attempts
// Hint: see 2 additional warnings for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch

#set page(numbering: "1", margin: (bottom: 20pt))
#show: doc => switch(n => {
  set page(supplement: "Pagus", numbering: "I") if n == 4
  doc
})

// Hint: 1-8 heading was created here
// Hint: 1-8 heading was created here
= Hello <hello>

// Warning: 2-28 supplement of the page on which the heading is located did not stabilize
// Warning: 2-28 numbering of the page on which the heading is located did not stabilize
// Hint: 2-28 the following supplements were observed:\n- run 1: `[]`\n- run 2: `[page]`\n- run 3: `[page]`\n- run 4: `[page]`\n- run 5: `[Pagus]`\n- final: `[page]`
// Hint: 2-28 the following numberings were observed:\n- run 1: `none`\n- run 2: `"1"`\n- run 3: `"1"`\n- run 4: `"1"`\n- run 5: `"I"`\n- final: `"1"`
#ref(<hello>, form: "page")

--- convergence-state ---
// Warning: document did not converge within five attempts
// Hint: see 2 additional warnings for more details
// Hint: see https://typst.app/help/convergence for help

#let hi = state("hi", 0)
#context hi.update(hi.get() + 1)
#context hi.update(hi.get() + 2)
#context hi.update(hi.get() + 3)
#context hi.update(hi.get() + 4)
#context hi.update(hi.get() + 5)

// Warning: 20-28 value of `state("hi")` did not converge
// Hint: 20-28 the following values were observed:\n- run 1: `0`\n- run 2: `5`\n- run 3: `9`\n- run 4: `12`\n- run 5: `14`\n- final: `15`
// Hint: 20-28 see https://typst.app/help/state-convergence for help
#context hi.update(hi.get() + 6)

#let s = state("s", 1)

// Warning: 19-28 value of `state("s")` did not converge
// Hint: 19-28 the following values were observed:\n- run 1: `1`\n- run 2: `2`\n- run 3: `3`\n- run 4: `4`\n- run 5: `5`\n- final: `6`
// Hint: 19-28 see https://typst.app/help/state-convergence for help
#context s.update(s.final() + 1)

--- convergence-state-errored ---
// Warning: document did not converge within five attempts
// Hint: see 1 additional warning for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch
#let s = state("s")
#switch(n => s.update(if n == 5 { _ => panic() } else { "ok" }))

// Warning: 16-23 value of `state("s")` did not converge
// Hint: 16-23 the following values were observed:\n- run 1: `none`\n- run 2: `"ok"`\n- run 3: `"ok"`\n- run 4: `"ok"`\n- run 5: `"ok"`\n- final: (errored)
// Hint: 16-23 see https://typst.app/help/state-convergence for help
#context { _ = s.get() }

--- convergence-counter ---
// Warning: document did not converge within five attempts
// Hint: see 2 additional warnings for more details
// Hint: see https://typst.app/help/convergence for help
#let c = counter("hi")

// Warning: 16-27 value of `counter("hi")` did not converge
// Hint: 16-27 the following values were observed:\n- run 1: 0\n- run 2: 0, 1\n- run 3: 0, 1, 3\n- run 4: 0, 1, 3, 7\n- run 5: 0, 1, 3, 7, 15\n- final: 0, 1, 3, 7, 15, 31
#context { _ = c.at(<end>) }

#context c.update({
  // Warning: 11-20 value of `counter("hi")` did not converge
  // Hint: 11-20 the following values were observed:\n- run 1: 0\n- run 2: 0, 1\n- run 3: 0, 1, 3\n- run 4: 0, 1, 3, 7\n- run 5: 0, 1, 3, 7, 15\n- final: 0, 1, 3, 7, 15, 31
  let v = c.final()
  v + (1 + v.last() * 2,)
})

#metadata(none) <end>

--- converge-bibliography-1 ---
// Warning: document did not converge within five attempts
// Hint: see 1 additional warning for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch
#switch(n => if n >= 5 { bibliography("/assets/bib/works.bib") })

// Error: 1-8 label `<netwok>` does not exist in the document
// Warning: 1-8 number of bibliography elements did not stabilize
// Hint: 1-8 the following numbers of elements were observed:\n- run 1: 0\n- run 2: 0\n- run 3: 0\n- run 4: 0\n- run 5: 0\n- final: 1
@netwok

--- converge-bibliography-2 ---
// Warning: document did not converge within five attempts
// Hint: see 2 additional warnings for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch

// Warning: 26-63 citation grouping did not stabilize
// Hint: 26-63 this can happen if the citations and bibliographies in the document did not stabilize by the end of the third layout iteration
#switch(n => if n >= 4 { bibliography("/assets/bib/works.bib") })

// Error: 1-8 cannot format citation in isolation
// Hint: 1-8 check whether this citation is measured without being inserted into the document
// Warning: 1-8 citation grouping did not stabilize
// Hint: 1-8 this can happen if the citations and bibliographies in the document did not stabilize by the end of the third layout iteration
@netwok

--- convergence-measure ---
// Warning: document did not converge within five attempts
// Hint: see 1 additional warning for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch
#switch(n => {
  // Hint: 15-22 the closest match for this element did not stabilize
  let body = [= Hello]

  // Warning: 7-20 a measured element did not stabilize
  // Hint: 7-20 measurement tries to resolve introspections by finding the closest matching elements in the real document
  _ = measure(body)
  if n == 4 {
    body
  }
})

--- convergence-html-id html ---
// Warning: document did not converge within five attempts
// Hint: see 1 additional warning for more details
// Hint: see https://typst.app/help/convergence for help
#import "switch.typ": switch
#switch(n => calc.min(n, 4) * [= Heading <a>])

// Error: 10-55 failed to determine link anchor
// Warning: 10-55 HTML element ID assigned to the destination heading did not stabilize
// Hint: 10-55 the following IDs were observed:\n- run 1: (no ID)\n- run 2: (no ID)\n- run 3: (no ID)\n- run 4: (no ID)\n- run 5: (no ID)\n- final: a-1
#context link(query(heading).last().location())[Hello]

--- convergence-state-converged-but-not-query ---
// In this example, the "high-level" state introspection yielded the same
// value in iteration 4 and 5, but the "low-level" state query yielded a
// different sequence. It also converged, but we don't know that until one
// iteration later.
#import "switch.typ": switch
#let s = state("a", none)
#switch(n => if n == 5 { s.update(none) })
#context s.get()
