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
// Warning: 13-21 label `<strike>` is not attached to anything
*This is* #[<strike>] *protected.*
*This is not.* <strike>

--- label-unclosed-is-text ---
// Test that incomplete label is text.
1 < 2 is #if 1 < 2 [not] a label.

--- label-text ---
// Test label on text.
#test([Hello<hi>].label, <hi>)

--- label-styled ---
// Test that label can bind to the content within styled content.
#let foo(x) = {
  show figure: it => {
    let number = counter(figure).display(it.numbering)
    [#number #it.body]
  }
  figure(x, supplement: "Foo", numbering: "1")
}

#foo[Hello World]<howdyy>
@howdyy

--- label-sequence ---
// Test that label can bind to content within a sequence.
#let foo(x) = {
  context counter(heading).get()
  figure(x, kind: "Foo", supplement: "Foo", numbering: "1")
}

#foo[Hello World]<howdy>
@howdy

--- label-sequence-styled-recurse ---
// Test that label traverses sequences and styled content recursively.
#let bar = {
  [word]
  align(end, heading[head])
  parbreak()
}

#let foo = {
  figure[fig]
  text(red, bar)
  parbreak()
}

#foo <uhoh>
// Error: 1-6 cannot reference align
@uhoh

--- label-unlabelled-element-field-access ---
// Test error message when trying to access "label" field.
// Error: 19-24 sequence does not have field "label"
#[#[A *B* C]<hi>].label

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

--- label-multiple-ignored-warn ---
// Warning: 1-8 content labelled multiple times
// Hint: 1-8 only the last label is used, the rest are ignored
= Hello <a> <b>

// Warning: 12-19 content labelled multiple times
// Hint: 12-19 only the last label is used, the rest are ignored
#let f = [#block()<c>]
#f<d>

// Warning: 6-13 content labelled multiple times
// Hint: 6-13 only the last label is used, the rest are ignored
#[#[#block()]<e>]<f>

// Error: 1-3 label `<a>` does not exist in the document
@a

--- label-unattached-warn ---
#set heading(numbering: "1.")
// Warning: 1-4 label `<a>` is not attached to anything
<a>
