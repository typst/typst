#show heading: set text(font: "Source Sans 3")
#set heading(numbering: "1.1")
= Fake Small Caps Tests
== Basic Functionality
=== Libertinus Serif
#table(
  columns: 2,
  [Default behavior], smallcaps[Hello World],
  [Forced synthesis], smallcaps(typographic: false)[Hello World],
)

=== Font Without Small Caps (IBM Plex Serif)
#set text(font: "IBM Plex Serif")
#table(
  columns: 2,
  [Default behavior], smallcaps[Hello World],
  [True typographic], smallcaps(typographic: true)[Hello World],
  [Forced synthesis], smallcaps(typographic: false)[Hello World],
)

== Unicode and Special Characters
#table(
  columns: 2,
  [Accented], smallcaps[Café naïve résumé],
  [German], smallcaps[Straße München],
  [Mixed], smallcaps[Hello 你好 World],
)

== Long Text (Justification Test)
#set par(justify: false)
#smallcaps(text(font: "Source Serif 4", lorem(50)))

#set par(justify: true)
#smallcaps(text(font: "Source Serif 4", lorem(50)))

#set par(justify: false)
#smallcaps(text(font: "IBM Plex Sans", lorem(50)))

#set par(justify: true)
#smallcaps(text(font: "IBM Plex Sans", lorem(50)))
