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
  if it.element != none and it.element.func() == figure {
    let element = it.element
    "["
    element.supplement
    "-"
    str(element.counter.at(element.location()).at(0))
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
  if it.element != none and it.element.func() == heading {
    let element = it.element
    "["
    emph(element.supplement)
    "-"
    numbering(element.numbering, ..counter(heading).at(element.location()))
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
  if it.element != none {
    if it.element.func() == text {
      let element = it.element
      "["
      element
      "]"
    } else if it.element.func() == underline {
      let element = it.element
      "{"
      element
      "}"
    } else {
      it
    }
  } else {
    it
  }
}

@txt

Ref something unreferable <txt>

@under
#underline[
Some underline text.
] <under>
