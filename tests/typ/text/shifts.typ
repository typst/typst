// Test sub- and superscipt shifts.

---
#table(columns: 3,
    [Typo.], [Fallb.], [Synth],
    [x#super[1]], [x#super[5n]], [x#super[2 #square(width: 6pt)]],
    [x#sub[1]], [x#sub[5n]], [x#sub[2 #square(width: 6pt)]],
)

---
#set super(typographic: false, baseline: -0.25em, size: 0.7em)
n#super[1], n#sub[2], ... n#super[N] 
