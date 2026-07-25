// Test references.

--- ref-basic paged html ---
#set heading(numbering: "1.")

= Introduction <intro>
See @setup.

== Setup <setup>
As seen in @intro, we proceed.

--- ref-label-missing paged ---
// Error: 1-5 label `<foo>` does not exist in the document
@foo

--- ref-label-duplicate paged ---
= First <foo>
= Second <foo>

// Error: 1-5 label `<foo>` occurs multiple times in the document
@foo

--- ref-within-label-path bundle ---
#set heading(numbering: "1.")

#[
  #document("alpha.pdf")[
    = #lorem(3) <heading-1>
    #[
      == #lorem(5) <subheading>
    ] <subscope-1>
  ] <doc-1>
  #document("beta.pdf")[
    = #lorem(4) <subheading>
  ] <doc-2>
] <scope>

#document("gamma.pdf")[
  @doc-1/subheading
  @subscope-1/subheading
  @doc-1/subscope-1/subheading
  #ref(<doc-1/subscope-1/subheading>)
  #ref(<doc-1>/<subscope-1>/<subheading>)

  #context test(str(<doc-1>/<subscope-1>), "doc-1/subscope-1")
  #context test(query(<doc-1/subscope-1/subheading>).len(), 1)
  #context test(query(<subscope-1/doc-1/subheading>).len(), 0)
]

--- ref-within-label-path-ambiguous bundle ---
#set heading(numbering: "1.")

#[
  #document("alpha.pdf")[
    = #lorem(3) <heading-1>
    #[
      == #lorem(5) <subheading>
    ] <subscope-1>
  ] <doc-1>
  #document("beta.pdf")[
    = #lorem(4) <subheading>
  ] <doc-2>
] <scope>

#document("gamma.pdf")[
  // Error: 3-14 label `<subheading>` occurs multiple times in the document
  @subheading

  // Error: 3-20 selector matches multiple elements
  @scope/subheading
]

--- ref-within-label-path-repeat bundle ---
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")

#let ct = [
  $ E = m c^2 $ <eq1>
  $ F = m a $ <eq2>
  #[= #lorem(2) <head>] <scope1>
  #[= #lorem(2) <head>] <scope2>
  - See @eq1, @eq2, @scope1/head, @scope2/head
  - Also look at @doc-a/eq1 and @doc-b/eq1.
]

#let prefix-reference(it, prefix: "") = if not str(it.target).contains("doc-") {
  ref(label(prefix + "/" + str(it.target)))
} else {
  it
}

#document("a.pdf")[
  #show ref: prefix-reference.with(prefix: "doc-a")
  #ct
] <doc-a>

#counter(heading).update(0)
#counter(math.equation).update(0)

#document("b.pdf")[
  #show ref: prefix-reference.with(prefix: "doc-b")
  #ct
] <doc-b>

--- ref-label-contains-paths eval ---
// Error: 11-27 label paths cannot be used to label content
= Heading <heading/syntax>

--- ref-supplements paged ---
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

--- ref-ambiguous paged ---
// Test ambiguous reference.
= Introduction <arrgh>

// Error: 1-7 label `<arrgh>` occurs both in the document and a bibliography
// Hint: 1-7 change either the heading's label or the bibliography key to resolve the ambiguity
@arrgh
#bibliography("/assets/bib/works.bib")

--- ref-form-page paged ---
#set page(numbering: "1")

Text <text> is on #ref(<text>, form: "page").
See #ref(<setup>, form: "page").

#set page(supplement: [p.])

== Setup <setup>
Text seen on #ref(<text>, form: "page").
Text seen on #ref(<text>, form: "page", supplement: "Page").

--- ref-form-page-unambiguous paged ---
// Test that page reference is not ambiguous.
#set page(numbering: "1")

= Introduction <arrgh>

#ref(<arrgh>, form: "page")
#bibliography("/assets/bib/works.bib")

--- ref-form-page-bibliography paged ---
// Error: 2-28 label `<quark>` does not exist in the document
#ref(<quark>, form: "page")
#bibliography("/assets/bib/works.bib")

--- issue-4536-non-whitespace-before-ref paged empty ---
// Test reference with non-whitespace before it.
#figure[] <1>
#test([(#ref(<1>))], [(@1)])

--- ref-to-empty-label-not-possible paged ---
// @ without any following label should just produce the symbol in the output
// and not produce a reference to a label with an empty name.
@

--- ref-function-empty-label eval ---
// using ref() should also not be possible
// Error: 6-7 unexpected less-than operator
// Error: 7-8 unexpected greater-than operator
#ref(<>)
