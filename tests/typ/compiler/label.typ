// Test labels.

---
// Test labelled headings.
#show heading: set text(10pt)
#show heading.where(label: <intro>): underline

= Introduction <intro>
The beginning.

= Conclusion
The end.

---
// Test label after expression.
#show strong.where(label: <v>): set text(red)

#let a = [*A*]
#let b = [*B*]
#a <v> #b

---
// Test labelled text.
#show "t": it => {
  set text(blue) if it.has("label") and it.label == <last>
  it
}

This is a thing #[that <last>] happened.

---
// Test abusing dynamic labels for styling.
#show <red>: set text(red)
#show <blue>: set text(blue)

*A* *B* <red> *C* #label("bl" + "ue") *D*

---
// Test that label ignores parbreak.
#show <hide>: none

_Hidden_
<hide>

_Hidden_

<hide>
_Visible_

---
// Test that label only works within one content block.
#show <strike>: strike
*This is* #[<strike>] *protected.*
*This is not.* <strike>

---
// Test that incomplete label is text.
1 < 2 is #if 1 < 2 [not] a label.

---
// Test label on text, styled, and sequence.
// Ref: false
#test([Hello<hi>].label, <hi>)
#test([#[A *B* C]<hi>].label, <hi>)
#test([#text(red)[Hello]<hi>].label, <hi>)
