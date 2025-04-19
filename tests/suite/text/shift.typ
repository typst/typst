// Test sub- and superscript shifts.

--- sub-super ---
#let sq = box(square(size: 4pt))
#table(
  columns: 3,
  [Typo.], [Fallb.], [Synth.],
  [x#super[1 #sq]], [x#super[5: #sq]], [x#super(typographic: false)[2 #sq]],
  [x#sub[1 #sq]], [x#sub[5: #sq]], [x#sub(typographic: false)[2 #sq]],
)

--- sub-super-typographic ---
#set text(size: 20pt)
// Libertinus Serif supports "subs" and "sups" for `typo`, but not for `synth`.
#let synth = [1,2,3]
#let typo = [123]
#let sq = [1#box(square(size: 4pt))2]
x#super(synth) x#super(typo) x#super(sq) \
x#sub(synth) x#sub(typo) x#sub(sq)

--- sub-super-italic-compensation ---
#set text(size: 20pt, style: "italic")
// Libertinus Serif supports "subs" and "sups" for `typo`, but not for `synth`.
#let synth = [1,2,3]
#let typo = [123]
#let sq = [1#box(square(size: 4pt))2]
x#super(synth) x#super(typo) x#super(sq) \
x#sub(synth) x#sub(typo) x#sub(sq)

--- sub-super-non-typographic ---
#set super(typographic: false, baseline: -0.25em, size: 0.7em)
n#super[1], n#sub[2], ... n#super[N]

--- super-underline ---
#set underline(stroke: 0.5pt, offset: 0.15em)
#set super(typographic: false)
#underline[The claim#super[4]] has been disputed. \
The claim#super[#underline[4]] has been disputed. \
The claim #underline(super[4]) has been disputed. \
#set super(typographic: true)
#underline[The claim#super[4]] has been disputed. \
The claim#super[#underline[4]] has been disputed. \
The claim #underline(super[4]) has been disputed.

--- super-highlight ---
#set super(typographic: false)
#highlight[The claim#super[4]] has been disputed. \
The claim#super[#highlight[4]] has been disputed. \
It really has been#super(highlight[4]) \
#set super(typographic: true)
#highlight[The claim#super[4]] has been disputed. \
The claim#super[#highlight[4]] has been disputed. \
It really has been#super(highlight[4])

--- super-1em ---
Test#super[#box(rect(height: 1em))]#box(rect(height: 1em))

--- long-scripts ---
|longscript| \
|#super(typographic: true)[longscript]| \
|#super(typographic: false)[longscript]| \
|#sub(typographic: true)[longscript]| \
|#sub(typographic: false)[longscript]|

--- scripts-with-bundeled-fonts ---
#let test(font, weights, styles) = {
  for weight in weights {
    for style in styles {
      text(font: font, weight: weight, style: style)[Xx#super[Xx]#sub[Xx]]
      linebreak()
    }
  }
}

#test("DejaVu Sans Mono", ("regular", "bold"), ("normal", "oblique"))
#test("Libertinus Serif", ("regular", "semibold", "bold"), ("normal", "italic"))
#test("New Computer Modern", ("regular", "bold"), ("normal", "italic"))
#test("New Computer Modern Math", (400, 450, "bold"), ("normal",))
