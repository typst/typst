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

#show ref: it => {
  if it.has("referee") and it.referee.func() == figure {
    let referee = it.referee
    "["
    referee.supplement
    "-"
    str(referee.counter.at(referee.location()).at(0))
    "]"
    // it
  } else {
    it
  }
}

#figure(
  image("/cylinder.svg", height: 3cm),
  caption: [A sylinder.],
  supplement: "Fig",
) <fig1>

#figure(
  image("/tiger.jpg", height: 3cm),
  caption: [A tiger.],
  supplement: "Figg",
) <fig2>

#figure(
  $ A = 1 $,
  kind: "equation",
  supplement: "Equa",

) <eq1>
@fig1

@fig2

@eq1

---
#set heading(numbering: (..nums) => {
  nums.pos().map(str).join(".")
  }, supplement: [Chapt])

#show ref: it => {
  if it.has("referee") and it.referee.func() == heading {
    let referee = it.referee
    "["
    emph(referee.supplement)
    "-"
    numbering(referee.numbering, ..counter(heading).at(referee.location()))
    "]"
  } else {
    it
  }
}

= Introduction <intro>

= Summary <sum>

== Subsection <sub>

@intro

@sum

@sub

---

#show ref: it => {
  if it.has("referee") and it.referee.func() == cite {
    let referee = it.referee
    "["
    referee.keys.at(0)
    "]"
  } else {
    it
  }
}

@arrgh

#bibliography("/works.bib")