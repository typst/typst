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
// Error: 18-22 expected content, found string
#show heading as "hi"
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
// Error: 2-16 set, show and wrap are only allowed directly in markup
{show list as a}
