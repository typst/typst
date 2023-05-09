// Tests outline 'indent' option.

---
// With heading numbering
#set page(width: 200pt)
#set heading(numbering: "1.a.")
#outline(indent: false)
#outline(indent: true)
#outline(indent: 2em)
#outline(indent: [--])
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
#outline(indent: false)
#outline(indent: true)
#outline(indent: 2em)
#outline(indent: [--])
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
