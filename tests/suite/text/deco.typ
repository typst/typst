// Test text decorations.

--- underline-overline-strike ---
#let red = rgb("fc0030")

// Basic strikethrough.
#strike[Statements dreamt up by the utterly deranged.]

// Move underline down.
#underline(offset: 5pt)[Further below.]

// Different color.
#underline(stroke: red, evade: false)[Critical information is conveyed here.]

// Inherits font color.
#text(fill: red, underline[Change with the wind.])

// Both over- and underline.
#overline(underline[Running amongst the wolves.])

--- strike-with ---
#let redact = strike.with(stroke: 10pt, extent: 0.05em)
#let highlight-custom = strike.with(stroke: 10pt + rgb("abcdef88"), extent: 0.05em)

// Abuse thickness and transparency for redacting and highlighting stuff.
Sometimes, we work #redact[in secret].
There might be #highlight-custom[redacted] things.

--- underline-stroke-folding ---
// Test stroke folding.
#set underline(stroke: 2pt, offset: 2pt)
#underline(text(red, [DANGER!]))

--- underline-background ---
// Test underline background
#set underline(background: true, stroke: (thickness: 0.5em, paint: red, cap: "round"))
#underline[This is in the background]

--- overline-background ---
// Test overline background
#set overline(background: true, stroke: (thickness: 0.5em, paint: red, cap: "round"))
#overline[This is in the background]

--- strike-background ---
// Test strike background
#set strike(background: true, stroke: 5pt + red)
#strike[This is in the background]

--- highlight ---
// Test highlight.
This is the built-in #highlight[highlight with default color].
We can also specify a customized value
#highlight(fill: green.lighten(80%))[to highlight].

--- highlight-bounds ---
// Test default highlight bounds.
#highlight[ace],
#highlight[base],
#highlight[super],
#highlight[phone #sym.integral]

--- highlight-edges ---
// Test a tighter highlight.
#set highlight(top-edge: "x-height", bottom-edge: "baseline")
#highlight[ace],
#highlight[base],
#highlight[super],
#highlight[phone #sym.integral]

--- highlight-edges-bounds ---
// Test a bounds highlight.
#set highlight(top-edge: "bounds", bottom-edge: "bounds")
#highlight[abc]
#highlight[abc #sym.integral]

--- highlight-radius ---
// Test highlight radius
#highlight(radius: 3pt)[abc],
#highlight(radius: 1em)[#lorem(5)]

--- highlight-stroke ---
// Test highlight stroke
#highlight(stroke: 2pt + blue)[abc]
#highlight(stroke: (top: blue, left: red, bottom: green, right: orange))[abc]
#highlight(stroke: 1pt, radius: 3pt)[#lorem(5)]


--- highlight-inline-math ---
// Test highlight for inline math equation.
#highlight[$a$], #highlight[$a_n$], #highlight[$a_n b$]
#highlight[$a_n = C_0 a_(n-1)$]
#highlight[$1/2 < (x+1)/2$]

--- highlight-inline-math-multiline ---
#line(length: 100%)
#h(1fr)
#highlight[$a + b + c + d + e + f + g$]

--- highlight-block-math ---
// Test highlight for block math equation.
#highlight[$ sum_(k=1)^n k = (n(n+1)) / 2 $]

--- highlight-partial-in-block-math ---
// Test partial highlight in a block math equation.
$ sum_(k=1)^n k = #highlight[(n(n+1))] / 3 $

--- highlight-only-hspace ---
// highlight hspace
A#highlight[#h(1cm, weak: false)]B

--- highlight-hspace-mixed-with-math ---
// highlight with hspace and math
#highlight[$a#h(0.5cm)a_n#h(0.5cm)a_n b$]

--- overline-only-hspace ---
// overline with hspaced issues 1716
A#overline[#h(1cm)]B

--- overline-hspace-mixed-with-math ---
// overline with hspace and math
#overline[$a#h(0.5cm)a_n#h(0.5cm)a_n b$]

--- underline-only-hspace ---
// underline with hspaced issues 1716
A#underline[#h(1cm)]B

--- underline-hspace-mixed-with-math ---
// underline with hspaced and math
#underline[$a#h(0.5cm)a_n#h(0.5cm)a_n b$]

--- strike-inline-math ---
// Test strike for inline math equation.
#strike[$a$], #strike[$a_n$], #strike[$a_n b$]
#strike[$a_n = C_0 a_(n-1)$]
#strike[$1/2 < (x+1)/2$]

--- strike-block-math ---
// Test strike for block math equation.
#strike[$ sum_(k=1)^n k = (n(n+1)) / 2 $]

--- strike-partial-in-block-math ---
// Test partial strike in a block math equation.
$ sum_(k=1)^n k = #strike[(n(n+1))] / 3 $

--- underline-inline-math ---
// Test underline for inline math equation.
#underline[$a$], #underline[$a_n$], #underline[$a_n b$]
#underline[$a_n = C_0 a_(n-1)$]
#underline[$1/2 < (x+1)/2$]

--- underline-block-math ---
// Test underline for block math equation.
#underline[$ sum_(k=1)^n k = (n(n+1)) / 2 $]

--- underline-partial-in-block-math ---
// Test partial underline in a block math equation.
$ sum_(k=1)^n k = #underline[(n(n+1))] / 3 $

--- overline-inline-math ---
// Test overline for inline math equation.
#overline[$a$], #overline[$a_n$], #overline[$a_n b$]
#overline[$a_n = C_0 a_(n-1)$]
#overline[$1/2 < (x+1)/2$]

--- overline-block-math ---
// Test overline for block math equation.
#overline[$ sum_(k=1)^n k = (n(n+1)) / 2 $]

--- overline-partial-in-block-math ---
// Test partial overline in a block math equation.
$ sum_(k=1)^n k = #overline[(n(n+1))] / 3 $
