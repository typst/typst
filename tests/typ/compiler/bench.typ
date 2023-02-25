// Ref: false

// Configuration with `page` and `font` functions.
#set page(width: 450pt, margin: 1cm)

// There are variables and they can take normal values like strings, ...
#let city = "Berlin"

// ... but also "content" values. While these contain markup,
// they are also values and can be summed, stored in arrays etc.
// There are also more standard control flow structures, like #if and #for.
#let university = [*Technische Universität #city*]
#let faculty = [*Fakultät II, Institut for Mathematik*]

// The `box` function just places content into a rectangular container. When
// the only argument to a function is a content block, the parentheses can be
// omitted (i.e. `f[a]` is the same as `f([a])`).
#box[
  // Backslash adds a forced line break.
  #university \
  #faculty \
  Sekretariat MA \
  Dr. Max Mustermann \
  Ola Nordmann, John Doe
]
#align(right, box[*WiSe 2019/2020* \ Woche 3])

// Adds vertical spacing.
#v(6mm)

// If the last argument to a function is a content block, we can also place it
// behind the parentheses.
#align(center)[
  // Markdown-like syntax for headings.
  ==== 3. Übungsblatt Computerorientierte Mathematik II #v(4mm)
  *Abgabe: 03.05.2019* (bis 10:10 Uhr in MA 001) #v(4mm)
  *Alle Antworten sind zu beweisen.*
]

*1. Aufgabe* #align(right)[(1 + 1 + 2 Punkte)]

Ein _Binärbaum_ ist ein Wurzelbaum, in dem jeder Knoten ≤ 2 Kinder hat.
Die Tiefe eines Knotens _v_ ist die Länge des eindeutigen Weges von der Wurzel
zu _v_, und die Höhe von _v_ ist die Länge eines längsten (absteigenden) Weges
von _v_ zu einem Blatt. Die Höhe des Baumes ist die Höhe der Wurzel.

#v(6mm)
