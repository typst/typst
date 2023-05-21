// Tests outline 'indent' option.

---
// With heading numbering
#set page(width: 200pt)
#set heading(numbering: "1.a.")
#outline()
#outline(indent: false)
#outline(indent: true)
#outline(indent: none)
#outline(indent: auto)
#outline(indent: 2em)
#outline(indent: [--])
#outline(indent: ([::], 2em, [=====]))
#outline(indent: n => (1em, none, [==], [====]).at(n))
#outline(indent: n => [*!*] * calc.pow(2, n))

= About ACME Corp.

== History
#lorem(10)

== Products
#lorem(10)

=== Categories
#lorem(10)

==== General
#lorem(10)

---
// Without heading numbering
#set page(width: 200pt)
#outline()
#outline(indent: false)
#outline(indent: true)
#outline(indent: none)
#outline(indent: auto)
#outline(indent: 2em)
#outline(indent: [--])
#outline(indent: ([::], 2em, [=====]))
#outline(indent: n => (1em, none, [==], [====]).at(n))
#outline(indent: n => [*!*] * calc.pow(2, n))

= About ACME Corp.

== History
#lorem(10)

== Products
#lorem(10)

=== Categories
#lorem(10)

==== General
#lorem(10)

---
// Error: 18-20 indent array must have at least one element
#outline(indent: ())

---
// Error: 18-30 expected relative length, fraction, content, or none, found auto
#outline(indent: (auto, none))

---
// Error: 2-35 indent function must return 'none', a spacing length, or content
#outline(indent: n => (a: "dict"))

= Heading
