#set heading(numbering: none)
#set page("a7", margin: 20pt, numbering: "1")
#show heading.where(level: 1): set text(12pt)
#show heading.where(level: 2): set text(10pt)

#outline()

#set heading(numbering: "(1/a)")

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
