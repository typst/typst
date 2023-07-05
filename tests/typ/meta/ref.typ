// Test references.

---
#set heading(numbering: "1.")

= Introduction <intro>
See @setup.

== Setup <setup>
As seen in @intro, we proceed.

---
// Error: 1-5 label does not exist in the document
@foo

---
= First <foo>
= Second <foo>

// Error: 1-5 label occurs multiple times in the document
@foo

---
#set heading(numbering: "1.", supplement: [Chapter])
#set math.equation(numbering: "(1)", supplement: [Eq.])

= Intro
#figure(
  image("/files/cylinder.svg", height: 1cm),
  caption: [A cylinder.],
  supplement: "Fig",
) <fig1>

#figure(
  image("/files/tiger.jpg", height: 1cm),
  caption: [A tiger.],
  supplement: "Tig",
) <fig2>

$ A = 1 $ <eq1>

#set math.equation(supplement: none)
$ A = 1 $ <eq2>

@fig1, @fig2, @eq1, (@eq2)

#set ref(supplement: none)
@fig1, @fig2, @eq1, @eq2
