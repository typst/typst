// Test text decorations.

---
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

---
#let redact = strike.with(stroke: 10pt, extent: 0.05em)
#let highlight-custom = strike.with(stroke: 10pt + rgb("abcdef88"), extent: 0.05em)

// Abuse thickness and transparency for redacting and highlighting stuff.
Sometimes, we work #redact[in secret].
There might be #highlight-custom[redacted] things.

---
// Test stroke folding.
#set underline(stroke: 2pt, offset: 2pt)
#underline(text(red, [DANGER!]))

---
// Test highlight.
This is the built-in #highlight[highlight with default color].
We can also specify a customized value
#highlight(fill: green.lighten(80%))[to highlight].

---
// Test default highlight bounds.
#highlight[ace],
#highlight[base],
#highlight[super],
#highlight[phone #sym.integral]

---
// Test a tighter highlight.
#set highlight(top-edge: "x-height", bottom-edge: "baseline")
#highlight[ace],
#highlight[base],
#highlight[super],
#highlight[phone #sym.integral]

---
// Test a bounds highlight.
#set highlight(top-edge: "bounds", bottom-edge: "bounds")
#highlight[abc]
#highlight[abc #sym.integral]

---
// Test underline background
#set underline(background: true, stroke: (thickness: 0.5em, paint: red, cap: "round"))
#underline[This is in the background]

---
// Test overline background
#set overline(background: true, stroke: (thickness: 0.5em, paint: red, cap: "round"))
#overline[This is in the background]


---
// Test strike background
#set strike(background: true, stroke: 5pt + red)
#strike[This is in the background]
