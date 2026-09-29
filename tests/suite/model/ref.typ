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

--- ref-within-selector bundle ---
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
  #ref(<doc-1>/<subscope-1>/<subheading>)
  #ref(selector(<subheading>).within(
    selector(<subscope-1>).within(<doc-1>),
  ))

  #context test(type(<doc-1/subscope-1>), selector)
  #context test(
    repr(<doc-1>/<subscope-1>),
    "<subscope-1>.within(<doc-1>)",
  )
  #context test(query(<doc-1/subscope-1/subheading>).len(), 1)
  #context test(query(<subscope-1/doc-1/subheading>).len(), 0)
]

--- ref-within-selector-ambiguous bundle ---
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

  // Error: 3-17 selector does not match any element
  @doc-1/missing
]

--- ref-within-selector-repeat bundle ---
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")

#let ct = [
  $ E = m c^2 $ <eq1>
  $ F = m a $ <eq2>
  #[= #lorem(2) <head>] <scope1>
  #[= #lorem(2) <head>] <scope2>
  - See @eq1, @eq2, @scope1/head, @scope2/head
]

#let revoke = metadata("prefixed-reference")
#let prefix-reference(it, scope: none) = {
  if bibliography.title == revoke { return it }
  set bibliography(title: revoke)
  ref(it.target.within(scope))
}

#document("a.pdf")[
  #[
    #show ref: prefix-reference.with(scope: <doc-a>)
    #ct
  ]
  Furthermore, @doc-a/eq1 and @doc-b/eq1.
] <doc-a>

#counter(heading).update(0)
#counter(math.equation).update(0)

#document("b.pdf")[
  #[
    #show ref: prefix-reference.with(scope: <doc-b>)
    #ct
  ]
  Furthermore, @doc-a/eq1 and @doc-b/eq1.
] <doc-b>

--- ref-selector-empty-middle eval ---
// Error: 1-6 selector cannot contain empty components
@a//b

--- ref-selector-empty-trailing eval ---
// Error: 1-6 selector cannot contain empty components
@a/b/

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
