// Test show rules.

#set page("a8", footer: p => v(-5pt) + align(right, [#p]))

#let i = 1
#set heading(size: 100%)
#show heading(level, body) as {
  if level == 1 {
    v(10pt)
    underline(text(150%, blue)[{i}. #body])
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
#set heading(size: 100%, strong: false, block: false)
#show heading(a, b) as [B]
A [= Heading] C

---
// Error: 1-22 unexpected argument
#show heading() as []
= Heading

---
// Error: 1-28 expected template, found string
#show heading(_, _) as "hi"
= Heading

---
// Error: 1-29 show rule is recursive
#show strong(x) as strong(x)
*Hi*

---
// Error: 2-19 set, show and wrap are only allowed directly in markup
{show list(a) as b}
