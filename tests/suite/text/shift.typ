// Test sub- and superscript shifts.

--- sub-super ---
#table(
  columns: 3,
  [Typo.], [Fallb.], [Synth],
  [x#super[1]], [x#super[5n]], [x#super[2 #box(square(size: 6pt))]],
  [x#sub[1]], [x#sub[5n]], [x#sub[2 #box(square(size: 6pt))]],
)

--- sub-super-typographic ---
#set super(typographic: true)
#set sub(typographic: true)
#table(
  columns: 3,
  [Typo.], [Fallb.], [Synth],
  [x#super[1]], [x#super[5n]], [x#super[2 #box(square(size: 6pt))]],
  [x#sub[1]], [x#sub[5n]], [x#sub[2 #box(square(size: 6pt))]],
)

--- sub-super-non-typographic ---
#set super(typographic: false)
#set sub(typographic: false)
#table(
  columns: 3,
  [Typo.], [Fallb.], [Synth],
  [x#super[1]], [x#super[5n]], [x#super[2 #box(square(size: 6pt))]],
  [x#sub[1]], [x#sub[5n]], [x#sub[2 #box(square(size: 6pt))]],
)

--- sub-super-synthesized ---
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
