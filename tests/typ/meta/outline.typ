#set page("a7", margin: 20pt, numbering: "1")
#set heading(numbering: "(1/a)")
#show heading.where(level: 1): set text(12pt)
#show heading.where(level: 2): set text(10pt)

#outline()

= Einleitung
#lorem(12)

= Analyse
#lorem(10)

#[
  #set heading(outlined: false)
  == Methodik
  #lorem(6)
]

== Verarbeitung
#lorem(4)

== Programmierung
```rust
fn main() {
  panic!("in the disco");
}
```

==== Deep Stuff
Ok ...

#set heading(numbering: "(I)")

= #text(blue)[Zusammen]fassung
#lorem(10)

---
// Test heading outline func.
#outline()

#set heading(
  numbering: "1.1",
  outline: (numbering, content) =>
    text(style: "italic", numbering) + h(1.5em, weak: true) + text(blue, content)
)

= A Heading
== A Subheading

#heading(numbering: none)[No Numbering]
