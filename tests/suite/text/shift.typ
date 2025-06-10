// Test sub- and superscript shifts.

--- sub-super ---
#table(
  columns: 3,
  [Typo.], [Fallb.], [Synth],
  [x#super[1]], [x#super[5n]], [x#super[2 #box(square(size: 6pt))]],
  [x#sub[1]], [x#sub[5n]], [x#sub[2 #box(square(size: 6pt))]],
)

--- sub-super-non-typographic ---
#set super(typographic: false, baseline: -0.25em, size: 0.7em)
n#super[1], n#sub[2], ... n#super[N]

--- super-underline ---
#set underline(stroke: 0.5pt, offset: 0.15em)
#underline[The claim#super[\[4\]]] has been disputed. \
The claim#super[#underline[\[4\]]] has been disputed. \
It really has been#super(box(text(baseline: 0pt, underline[\[4\]]))) \

--- basic-super html ---
1#super[st], 2#super[nd], 3#super[rd].
