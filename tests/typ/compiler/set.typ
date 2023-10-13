// General tests for set.

---
// Test that text is affected by instantiation-site bold.
#let x = [World]
Hello *#x*

---
// Test that lists are affected by correct indents.
#let fruit = [
  - Apple
  - Orange
  #list(body-indent: 20pt)[Pear]
]

- Fruit
#[#set list(indent: 10pt)
 #fruit]
- No more fruit

---
// Test that that block spacing and text style are respected from
// the outside, but the more specific fill is respected.
#set block(spacing: 4pt)
#set text(style: "italic", fill: eastern)
#let x = [And the forest #parbreak() lay silent!]
#text(fill: forest, x)

---
// Test that scoping works as expected.
#{
  if true {
    set text(blue)
    [Blue ]
  }
  [Not blue]
}

---
// Test relative path resolving in layout phase.
#let choice = ("monkey.svg", "rhino.png", "tiger.jpg")
#set enum(numbering: n => {
  let path = "/files/" + choice.at(n - 1)
  move(dy: -0.15em, image(path, width: 1em, height: 1em))
})

+ Monkey
+ Rhino
+ Tiger

---
// Test conditional set.
#show ref: it => {
  set text(red) if it.target == <unknown>
  "@" + str(it.target)
}

@hello from the @unknown

---
// Error: 19-24 expected boolean, found integer
#set text(red) if 1 + 2

---
// Error: 12-26 set is only allowed directly in code and content blocks
#{ let x = set text(blue) }
