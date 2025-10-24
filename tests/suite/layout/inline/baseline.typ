// Test baseline handling.

--- baseline-text render ---
Hi #text(1.5em)[You], #text(0.75em)[how are you?]

Our cockatoo was one of the
#text(baseline: -0.2em)[#box(circle(radius: 2pt)) first]
#text(baseline: 0.2em)[birds #box(circle(radius: 2pt))]
that ever learned to mimic a human voice.

--- baseline-box render ---
Hey #box(baseline: 40%, image("/assets/images/tiger.jpg", width: 1.5cm)) there!

--- issue-2214-baseline-math render ---
// The math content should also be affected by the TextElem baseline.
hello #text(baseline: -5pt)[123 #sym.WW\orld]\
hello #text(baseline: -5pt)[$123 WW#text[or]$ld]\
