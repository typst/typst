// Test show rules.

---
// Override lists.
#show list: it => "(" + it.children.map(v => v.body).join(", ") + ")"

- A
  - B
  - C
- D
- E

---
// Test full reset.
#show heading: [B]
#show heading: set text(size: 10pt, weight: 400)
A #[= Heading] C

---
// Test full removal.
#show heading: none

Where is
= There are no headings around here!
my heading?

---
// Test integrated example.
#show heading: it => block({
  set text(10pt)
  box(move(dy: -1pt)[📖])
  h(5pt)
  if it.level == 1 {
    underline(text(1.25em, blue, it.body))
  } else {
    text(red, it.body)
  }
})

= Task 1
Some text.

== Subtask
Some more text.

= Task 2
Another text.

---
// Test set and show in code blocks.
#show heading: it => {
  set text(red)
  show "ding": [🛎]
  it.body
}

= Heading

---
// Test that scoping works as expected.
#{
  let world = [ World ]
  show "W": strong
  world
  {
    set text(blue)
    show: it => {
      show "o": "Ø"
      it
    }
    world
  }
  world
}

---
#show heading: [1234]
= Heading

---
// Error: 25-29 content does not contain field "page"
#show heading: it => it.page
= Heading

---
#show text: none
Hey

---
// Error: 7-12 only element functions can be used as selectors
#show upper: it => {}

---
// Error: 16-20 expected content or function, found integer
#show heading: 1234
= Heading

---
// Error: 7-10 expected symbol, string, label, function, regex, or selector, found color
#show red: []

---
// Error: 7-25 show is only allowed directly in code and content blocks
#(1 + show heading: none)
