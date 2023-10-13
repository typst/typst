
---
#set page(
  paper: "a7",
  numbering: "1 / 1",
  margin: (bottom: 1cm, rest: 0.5cm),
)

#show heading.where(level: 1, outlined: true): it => [
  #it

  #set text(size: 12pt, weight: "regular")
  #outline(
    title: "Chapter outline",
    indent: true,
    target: heading
      .where(level: 1)
      .or(heading.where(level: 2))
      .after(it.location(), inclusive: true)
      .before(
        heading
          .where(level: 1, outlined: true)
          .after(it.location(), inclusive: false),
        inclusive: false,
      )
  )
]

#set heading(outlined: true, numbering: "1.")

= Section 1
== Subsection 1
== Subsection 2
=== Subsubsection 1
=== Subsubsection 2
== Subsection 3

= Section 2
== Subsection 1
== Subsection 2

= Section 3
== Subsection 1
== Subsection 2
=== Subsubsection 1
=== Subsubsection 2
=== Subsubsection 3
== Subsection 3

---

#set page(
  paper: "a7",
  numbering: "1 / 1",
  margin: (bottom: 1cm, rest: 0.5cm),
)

#set heading(outlined: true, numbering: "1.")

// This is purposefully an empty
#locate(loc => [
  Non-outlined elements:
  #(query(selector(heading).and(heading.where(outlined: false)), loc)
    .map(it => it.body).join(", "))
])

#heading("A", outlined: false)
#heading("B", outlined: true)
#heading("C", outlined: true)
#heading("D", outlined: false)
