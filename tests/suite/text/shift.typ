// Test sub- and superscript shifts.

--- sub-super paged ---
#let sq = box(square(size: 4pt))
#table(
  columns: 3,
  [Typo.], [Fallb.], [Synth.],
  [x#super[1#sq]], [x#super[5: #sq]], [x#super(typographic: false)[2 #sq]],
  [x#sub[1#sq]], [x#sub[5: #sq]], [x#sub(typographic: false)[2 #sq]],
)

--- sub-super-typographic paged ---
#set text(size: 20pt)
// Libertinus Serif supports "subs" and "sups" for `typo` and `sq`, but not for
// `synth`.
#let synth = [1,2,3]
#let typo = [123]
#let sq = [1#box(square(size: 4pt))2]
x#super(synth) x#super(typo) x#super(sq) \
x#sub(synth) x#sub(typo) x#sub(sq)

--- sub-super-italic-compensation paged ---
#set text(size: 20pt, style: "italic")
// Libertinus Serif supports "subs" and "sups" for `typo`, but not for `synth`.
#let synth = [1,2,3]
#let typo = [123]
#let sq = [1#box(square(size: 4pt))2]
x#super(synth) x#super(typo) x#super(sq) \
x#sub(synth) x#sub(typo) x#sub(sq)

--- sub-super-non-typographic paged ---
#set super(typographic: false, baseline: -0.25em, size: 0.7em)
n#super[1], n#sub[2], ... n#super[N]

--- super-underline paged ---
#set underline(stroke: 0.5pt, offset: 0.15em)
#set super(typographic: false)
#underline[A#super[4]] B \
A#super[#underline[4]] B \
A #underline(super[4]) B \
#set super(typographic: true)
#underline[A#super[4]] B \
A#super[#underline[4]] B \
A #underline(super[4]) B

--- super-highlight paged ---
#set super(typographic: false)
#highlight[A#super[4]] B \
A#super[#highlight[4]] B \
A#super(highlight[4]) \
#set super(typographic: true)
#highlight[A#super[4]] B \
A#super[#highlight[4]] B \
A#super(highlight[4])

--- super-1em paged ---
#set text(size: 10pt)
#super(context test(1em.to-absolute(), 10pt))

--- long-scripts paged ---
|longscript| \
|#super(typographic: true)[longscript]| \
|#super(typographic: false)[longscript]| \
|#sub(typographic: true)[longscript]| \
|#sub(typographic: false)[longscript]|

--- script-metrics-bundled-fonts paged ---
// Tests whether the script metrics are used properly by synthesizing
// superscripts and subscripts for all bundled fonts.

#set super(typographic: false)
#set sub(typographic: false)

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

--- basic-sup-sub html ---
1#super[st], 2#super[nd], 3#super[rd].

log#sub[2], log#sub[3], log#sub[variable].

--- issue-7249-multiple-lookup-tables paged ---
// We increase the font size to make sure the difference is visible in the
// low-resolution reference image.
#set text(font: "Source Serif 4", size: 1.5em)
#set super(typographic: true)

A#super[test] \
A#super[test1] \
A#super[(test)] \
// Source Serif 4 does not support `sups` for backticks, so this should be
// synthesized.
A#super[test\`]
