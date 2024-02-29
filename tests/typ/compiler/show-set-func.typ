// Test set rules on an element in show rules for said element.

---
// These are both red because in the expanded form, `set text(red)` ends up
// closer to the content than `set text(blue)`.
#show strong: it => { set text(red); it }
Hello *World*

#show strong: it => { set text(blue); it }
Hello *World*

---
// This doesn't have an effect. An element is materialized before any show
// rules run.
#show heading: it => { set heading(numbering: "(I)"); it }
= Heading
