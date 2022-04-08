// Test show rules.

#set page("a8", footer: p => v(-5pt) + align(right, [#p]))

#let i = 1
#set heading(size: 1em)
#show heading(level, body) as {
  if level == 1 {
    v(10pt)
    underline(text(1.5em, blue)[{i}. #body])
    i += 1
  } else {
    text(red, body)
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
#set heading(size: 1em, strong: false, block: false)
#show heading(a, b) as [B]
A [= Heading] C

---
// Error: 14-22 unexpected argument
#show heading() as []
= Heading

---
// Error: 14-28 expected content, found string
#show heading(_, _) as "hi"
= Heading

---
// Error: 7-12 this function cannot be customized with show
#show upper() as {}

---
// Ref: false
// // Error: 1-29 show rule is recursive
// #show strong(x) as strong(x)
// *Hi*

---
// Error: 2-19 set, show and wrap are only allowed directly in markup
{show list(a) as b}
