// Test labels.

--- label-show-where-selector ---
// Test labelled headings.
#show heading: set text(10pt)
#show heading.where(label: <intro>): underline

= Introduction <intro>
The beginning.

= Conclusion
The end.

--- label-after-expression ---
// Test label after expression.
#show strong.where(label: <v>): set text(red)

#let a = [*A*]
#let b = [*B*]
#a <v> #b

--- label-dynamic-show-set ---
// Test abusing dynamic labels for styling.
#show <red>: set text(red)
#show <blue>: set text(blue)

*A* *B* <red> *C* #label("bl" + "ue") *D*

--- label-after-parbreak ---
// Test that label ignores parbreak.
#show <hide>: none

_Hidden_
<hide>

_Hidden_

<hide>
_Visible_

--- label-in-block ---
// Test that label only works within one content block.
#show <strike>: strike
*This is* #[<strike>] *protected.*
*This is not.* <strike>

--- label-unclosed-is-text ---
// Test that incomplete label is text.
1 < 2 is #if 1 < 2 [not] a label.

--- label-text-styled-and-sequence ---
// Test label on text, styled, and sequence.
#test([Hello<hi>].label, <hi>)
#test([#[A *B* C]<hi>].label, <hi>)
#test([#text(red)[Hello]<hi>].label, <hi>)

--- label-string-conversion ---
// Test getting the name of a label.
#test(str(<hey>), "hey")
#test(str(label("hey")), "hey")
#test(str([Hmm<hey>].label), "hey")

--- label-in-code-mode-hint ---
// Error: 7-7 expected semicolon or line break
// Hint: 7-7 labels can only be applied in markup mode
// Hint: 7-7 try wrapping your code in a markup block (`[ ]`)
#{ [A] <a> }
