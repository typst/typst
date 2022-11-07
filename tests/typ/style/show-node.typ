// Test node show rules.

---
// Override lists.
#set list(around: none)
#show list: it => "(" + it.items.join(", ") + ")"

- A
  - B
  - C
- D
- E

---
// Test full reset.
#set heading(size: 1em, strong: false, around: none)
#show heading: [B]
A [= Heading] C

---
// Test full removal.
#show heading: none
#set heading(around: none)

Where is
= There are no headings around here!
my heading?

---
// Test integrated example.
#set heading(size: 1em)
#show heading: it => {
  move(dy: -1pt)[📖]
  h(5pt)
  if it.level == 1 {
    underline(text(1.25em, blue, it.body))
  } else {
    text(red, it.body)
  }
}

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
{
  let world = [ World ]
  show "W": strong
  world
  {
    set text(blue)
    show it => {
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
// Error: 25-29 unknown field "page"
#show heading: it => it.page
= Heading

---
// Error: 7-12 this function cannot be customized with show
#show upper: it => {}

---
// Error: 16-20 expected content or function, found integer
#show heading: 1234
= Heading

---
// Error: 7-10 expected selector, found color
#show red: []

---
// Error: 7-25 show is only allowed directly in code and content blocks
{ 1 + show heading: none }
