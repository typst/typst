// Test node show rules.

---
// Override lists.
#set list(around: none)
#show v: list as "(" + v.items.join(", ") + ")"

- A
  - B
  - C
- D
- E

---
// Test full reset.
#set heading(size: 1em, strong: false, around: none)
#show heading as [B]
A [= Heading] C

---
// Test full removal.
#show heading as []
#set heading(around: none)

Where is
= There are not headings around here!
my heading?

---
// Test integrated example.
#set heading(size: 1em)
#show node: heading as {
  move(dy: -1pt)[📖]
  h(5pt)
  if node.level == 1 {
    underline(text(1.25em, blue, node.body))
  } else {
    text(red, node.body)
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
#show node: heading as {
  set text(red)
  show "ding" as [🛎]
  node.body
}

= Heading

---
// Test that scoping works as expected.
{
  let world = [ World ]
  show c: "W" as strong(c)
  world
  {
    set text(blue)
    wrap it in {
      show "o" as "Ø"
      it
    }
    world
  }
  world
}

---
#show heading as 1234
= Heading

---
// Error: 25-29 unknown field "page"
#show it: heading as it.page
= Heading

---
// Error: 10-15 this function cannot be customized with show
#show _: upper as {}

---
// Error: 7-10 expected function, string or regular expression, found color
#show red as []

---
// Error: 7-27 show is only allowed directly in code and content blocks
{ 1 + show heading as none }
