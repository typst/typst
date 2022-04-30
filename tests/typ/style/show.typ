// Test show rules.

#set page("a8", footer: p => v(-5pt) + align(right, [#p]))

#let i = 1
#set heading(size: 1em)
#show node: heading as {
  if node.level == 1 {
    v(10pt)
    underline(text(1.5em, blue)[{i}. {node.body}])
    i += 1
  } else {
    text(red, node.body)
  }
}

#v(-10pt)

= Aufgabe
Some text.

== Subtask
Some more text.

== Another subtask
Some more text.

= Aufgabe
Another text.

---
#set heading(size: 1em, strong: false, around: none)
#show _: heading as [B]
A [= Heading] C

---
// Error: 21-25 expected content, found string
#show _: heading as "hi"
= Heading

---
// Error: 22-29 dictionary does not contain key: "page"
#show it: heading as it.page
= Heading

---
// Error: 10-15 this function cannot be customized with show
#show _: upper as {}

---
// Error: 2-19 set, show and wrap are only allowed directly in markup
{show a: list as a}
