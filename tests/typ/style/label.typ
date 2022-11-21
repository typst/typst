// Test labels.

---
// Test labelled headings.
#show heading: set text(10pt)
#show heading.where(label: "intro"): underline

= Introduction <intro>
The beginning.

= Conclusion
The end.

---
// Test label after expression.
#show strong.where(label: "v"): set text(red)

#let a = [*A*]
#let b = [*B*]
#a <v> #b

---
// Test labelled text.
#show "t": it => {
  set text(blue) if it.label == "last"
  it
}

This is a thing [that <last>] happened.

---
// Test abusing labels for styling.
#show strong.where(label: "red"): set text(red)
#show strong.where(label: "blue"): set text(blue)

*A* *B* <red> *C* <blue> *D*

---
// Test that label ignores parbreak.
#show emph.where(label: "hide"): none

_Hidden_
<hide>

_Hidden_

<hide>
_Visible_

---
// Test that label only works within one content block.
#show strong.where(label: "strike"): strike
*This is* [<strike>] *protected.*
*This is not.* <strike>
