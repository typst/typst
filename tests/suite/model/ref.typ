// Test references.

--- ref-basic ---
#set heading(numbering: "1.")

= Introduction <intro>
See @setup.

== Setup <setup>
As seen in @intro, we proceed.

--- ref-label-missing ---
// Error: 1-5 label `<foo>` does not exist in the document
@foo

--- ref-label-duplicate ---
= First <foo>
= Second <foo>

// Error: 1-5 label `<foo>` occurs multiple times in the document
@foo

--- ref-supplements ---
#set heading(numbering: "1.", supplement: [Chapter])
#set math.equation(numbering: "(1)", supplement: [Eq.])

= Intro
#figure(
  image("/assets/images/cylinder.svg", height: 1cm),
  caption: [A cylinder.],
  supplement: "Fig",
) <fig1>

#figure(
  image("/assets/images/tiger.jpg", height: 1cm),
  caption: [A tiger.],
  supplement: "Tig",
) <fig2>

$ A = 1 $ <eq1>

#set math.equation(supplement: none)
$ A = 1 $ <eq2>

@fig1, @fig2, @eq1, (@eq2)

#set ref(supplement: none)
@fig1, @fig2, @eq1, @eq2

--- ref-ambiguous ---
// Test ambiguous reference.
= Introduction <arrgh>

// Error: 1-7 label occurs in the document and its bibliography
@arrgh
#bibliography("/assets/bib/works.bib")

--- ref-form-page ---
#set page(numbering: "1")

Text <text> is on #ref(<text>, form: "page").
See #ref(<setup>, form: "page").

#set page(supplement: [p.])

== Setup <setup>
Text seen on #ref(<text>, form: "page").
Text seen on #ref(<text>, form: "page", supplement: "Page").

--- ref-form-page-unambiguous ---
// Test that page reference is not ambiguous.
#set page(numbering: "1")

= Introduction <arrgh>

#ref(<arrgh>, form: "page")
#bibliography("/assets/bib/works.bib")

--- ref-form-page-bibliography ---
// Error: 2-28 label `<quark>` does not exist in the document
#ref(<quark>, form: "page")
#bibliography("/assets/bib/works.bib")

--- issue-4536-non-whitespace-before-ref ---
// Test reference with non-whitespace before it.
#figure[] <1>
#test([(#ref(<1>))], [(@1)])

--- ref-to-empty-label-not-possible ---
// @ without any following label should just produce the symbol in the output
// and not produce a reference to a label with an empty name.
@ \<- this should show up

--- ref-function-can-not-form-empty-label ---
// using ref() should also not be possible
// Error: 6-7 unexpected less-than operator
// Error: 7-8 unexpected greater-than operator
#ref(<>)
