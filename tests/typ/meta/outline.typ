#set page("a7", margin: 20pt, numbering: "1")
#set heading(numbering: "(1/a)")
#show heading.where(level: 1): set text(12pt)
#show heading.where(level: 2): set text(10pt)
#set math.equation(numbering: "1")

#outline()
#outline(title: [Figures], target: figure)
#outline(title: [Equations], target: math.equation)

= Introduction
#lorem(12)

= Analysis
#lorem(10)

#[
  #set heading(outlined: false)
  == Methodology
  #lorem(6)
]

== Math
$x$ is a very useful constant. See it in action:
$ x = x $

== Interesting figures
#figure(rect[CENSORED], kind: image, caption: [A picture showing a programmer at work.])
#figure(table[1x1], caption: [A very small table.])

== Programming
```rust
fn main() {
  panic!("in the disco");
}
```

==== Deep Stuff
Ok ...

// Ensure 'bookmarked' option doesn't affect the outline
#set heading(numbering: "(I)", bookmarked: false)

= #text(blue)[Sum]mary
#lorem(10)
