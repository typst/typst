// Test baseline handling.

--- baseline-text paged ---
Hi #text(1.5em)[You], #text(0.75em)[how are you?]

Our cockatoo was one of the
#text(baseline: -0.2em)[#box(circle(radius: 2pt)) first]
#text(baseline: 0.2em)[birds #box(circle(radius: 2pt))]
that ever learned to mimic a human voice.

--- baseline-box paged ---
Hey #box(baseline: 40%, image("/assets/images/tiger.jpg", width: 1.5cm)) there!


--- baseline-inline-math paged ---
#bounds[$sum_(i = 1)^n i/pi = 10$]

--- baseline-display-math paged ---
#bounds[$ sum_(i = 1)^n i/pi = 10 $]
#bounds[$ (x + y + z)/(2 dot.c 3) $]
#bounds({
  set math.equation(numbering: "(1)")
  $ sum_(i = 1)^n i/pi = 10 $
})

--- baseline-breakable-display-math paged ---
#set page(height: 5em)
#show math.equation: set block(breakable: true)

#bounds[$
  sum_(i = 1)^n i/pi &= 1 \
  sum_(i = 1)^n i/pi &= 2 \
  sum_(i = 1)^n i/pi &= 3
$]

--- baseline-paragraph paged ---
#bounds[
  Hello world!

  Bye!
]
#bounds[#text(2em)[world!] Hello!

  Cya!
]

--- baseline-isolated-box paged ---
#bounds(box(baseline: 5em)[Hello!])

#bounds[Hello! #box(baseline: 5em)[Hello!] Hello!]

--- baseline-move paged ---
#bounds(move(dy: 1em)[Test])
#bounds[Hello #box(move(dy: 1em)[Test])]
#bounds[Hello #box(move(dy: 1em)[Test]) #box(move(dy: 2em)[Testt])]

--- issue-2214-baseline-math paged ---
// The math content should also be affected by the TextElem baseline.
hello #text(baseline: -5pt)[123 #sym.WW\orld]\
hello #text(baseline: -5pt)[$123 WW#text[or]$ld]\
