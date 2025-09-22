// Test configuring paragraph properties.

--- par-basic render pdftags ---
#set page(width: 250pt, height: 120pt)

But, soft! what light through yonder window breaks? It is the east, and Juliet
is the sun. Arise, fair sun, and kill the envious moon, Who is already sick and
pale with grief, That thou her maid art far more fair than she: Be not her maid,
since she is envious; Her vestal livery is but sick and green And none but fools
do wear it; cast it off. It is my lady, O, it is my love! O, that she knew she
were! She speaks yet she says nothing: what of that? Her eye discourses; I will
answer it.

I am too bold, 'tis not to me she speaks: Two of the fairest stars in all the
heaven, Having some business, do entreat her eyes To twinkle in their spheres
till they return. What if her eyes were there, they in her head? The brightness
of her cheek would shame those stars, As daylight doth a lamp; her eyes in
heaven Would through the airy region stream so bright That birds would sing and
think it were not night. See, how she leans her cheek upon her hand! O, that I
were a glove upon that hand, That I might touch that cheek!

--- par-semantic ---
#show par: highlight

I'm a paragraph.

#align(center, table(
  columns: 3,

  // No paragraphs.
  [A],
  block[B],
  block[C *D*],

  // Paragraphs.
  par[E],
  [

    F
  ],
  [
    G

  ],

  // Paragraphs.
  parbreak() + [H],
  [I] + parbreak(),
  parbreak() +  [J] + parbreak(),

  // Paragraphs.
  [K #v(10pt)],
  [#v(10pt) L],
  [#place[] M],

  // Paragraphs.
  [
    N

    O
  ],
  [#par[P]#par[Q]],
  // No paragraphs.
  [#block[R]#block[S]],
))

--- par-semantic-html html ---
= Heading is no paragraph

I'm a paragraph.

#html.elem("div")[I'm not.]

#html.elem("div")[
  We are two.

  So we are paragraphs.
]

--- par-semantic-tag ---
#show par: highlight
#block[
  #metadata(none) <hi1>
  A
  #metadata(none) <hi2>
]

#block(width: 100%, metadata(none) + align(center)[A])
#block(width: 100%, align(center)[A] + metadata(none))

--- par-semantic-align ---
#show par: highlight
#show bibliography: none
#set block(width: 100%, stroke: 1pt, inset: 5pt)

#bibliography("/assets/bib/works.bib")

#block[
  #set align(right)
  Hello
]

#block[
  #set align(right)
  Hello
  @netwok
]

#block[
  Hello
  #align(right)[World]
  You
]

#block[
  Hello
  #align(right)[@netwok]
  You
]

--- par-leading-and-spacing ---
// Test changing leading and spacing.
#set par(spacing: 1em, leading: 2pt)
But, soft! what light through yonder window breaks?

It is the east, and Juliet is the sun.

--- par-spacing-context ---
#set par(spacing: 10pt)
#context test(par.spacing, 10pt)

--- par-first-line-indent ---
#set par(first-line-indent: 12pt, spacing: 5pt, leading: 5pt)
#show heading: set text(size: 10pt)

The first paragraph has no indent.

But the second one does.

#box(image("/assets/images/tiger.jpg", height: 6pt))
starts a paragraph, also with indent.

#align(center, image("/assets/images/rhino.png", width: 1cm))

= Headings
- And lists.
- Have no indent.

  Except if you have another paragraph in them.

#set text(8pt, lang: "ar", font: ("Noto Sans Arabic", "Libertinus Serif"))
#set par(leading: 8pt)

= Arabic
دع النص يمطر عليك

ثم يصبح النص رطبًا وقابل للطرق ويبدو المستند رائعًا.

--- par-first-line-indent-all ---
#set par(
  first-line-indent: (amount: 12pt, all: true),
  spacing: 5pt,
  leading: 5pt,
)
#set block(spacing: 1.2em)
#show heading: set text(size: 10pt)

= Heading
All paragraphs are indented.

Even the first.

--- par-first-line-indent-all-list ---
#show list.where(tight: false): set list(spacing: 1.2em)
#set par(
  first-line-indent: (amount: 12pt, all: true),
  spacing: 5pt,
  leading: 5pt,
)

- A #parbreak() B #line(length: 100%) C

- D

--- par-first-line-indent-all-enum ---
#show enum.where(tight: false): set enum(spacing: 1.2em)
#set par(
  first-line-indent: (amount: 12pt, all: true),
  spacing: 5pt,
  leading: 5pt,
)

+ A #parbreak() B #line(length: 100%) C

+ D

--- par-first-line-indent-all-terms render pdftags ---
#show terms.where(tight: false): set terms(spacing: 1.2em)
#set terms(hanging-indent: 10pt)
#set par(
  first-line-indent: (amount: 12pt, all: true),
  spacing: 5pt,
  leading: 5pt,
)

/ Term A: B \ C #parbreak() D #line(length: 100%) E

/ Term F: G

--- par-spacing-and-first-line-indent ---
// This is madness.
#set par(first-line-indent: 12pt)
Why would anybody ever ...

... want spacing and indent?

--- par-hanging-indent ---
// Test hanging indent.
#set par(hanging-indent: 15pt, justify: true)
#lorem(10)

--- par-hanging-indent-semantic ---
#set par(hanging-indent: 15pt)
= I am not affected

I am affected by hanging indent.

--- par-hanging-indent-manual-linebreak ---
#set par(hanging-indent: 1em)
Welcome \ here. Does this work well?

--- par-hanging-indent-rtl ---
#set par(hanging-indent: 2em)
#set text(dir: rtl, font: ("Libertinus Serif", "Noto Sans Arabic"))
لآن وقد أظلم الليل وبدأت النجوم
تنضخ وجه الطبيعة التي أعْيَتْ من طول ما انبعثت في النهار

--- par-trailing-whitespace ---
// Ensure that trailing whitespace layouts as intended.
#box(fill: aqua, " ")

--- par-contains-parbreak ---
#par[
  Hello
  // Warning: 4-14 parbreak may not occur inside of a paragraph and was ignored
  #parbreak()
  World
]

--- par-contains-block ---
#par[
  Hello
  // Warning: 4-11 block may not occur inside of a paragraph and was ignored
  #block[]
  World
]

--- par-empty-metadata ---
// Check that metadata still works in a zero length paragraph.
#block(height: 0pt)[#""#metadata(false)<hi>]
#context test(query(<hi>).first().value, false)

--- par-metadata-after-trimmed-space ---
// Ensure that metadata doesn't prevent trailing spaces from being trimmed.
#set par(justify: true, linebreaks: "simple")
#set text(hyphenate: false)
Lorem ipsum dolor #metadata(none) nonumy eirmod tempor.

--- par-show-children ---
// Variant 1: Prevent recursion by checking the children.
#let p = counter("p")
#let step = p.step()
#let nr = context p.display()
#show par: it => {
  if it.body.at("children", default: ()).at(0, default: none) == step {
    return it
  }
  par(step + [§#nr ] + it.body)
}

= A

B

C #parbreak() D

#block[E]

#block[F #parbreak() G]

--- par-show-styles ---
// Variant 2: Prevent recursion by observing a style.
#let revoke = metadata("revoke")
#show par: it => {
  if bibliography.title == revoke { return it }
  set bibliography(title: revoke)
  let p = counter("p")
  par[#p.step()§#context p.display() #it.body]
}

= A

B

C

--- par-explicit-trim-space ---
A

#par[ B ]

--- issue-4278-par-trim-before-equation ---
#set par(justify: true)
#lorem(6) aa $a = c + b$

--- issue-4938-par-bad-ratio ---
#set par(justify: true)
#box($k in NN_0$)

--- issue-4770-par-tag-at-start ---
#h(0pt) #box[] <a>

#context test(query(<a>).len(), 1)

--- issue-5831-par-constructor-args ---
// Make sure that all arguments are also respected in the constructor.
A
#par(
  leading: 2pt,
  spacing: 20pt,
  justify: true,
  linebreaks: "simple",
  first-line-indent: (amount: 1em, all: true),
  hanging-indent: 5pt,
)[
  The par function has a constructor and justification.
]

--- show-par-set-block-hint ---
// Warning: 2-36 `show par: set block(spacing: ..)` has no effect anymore
// Hint: 2-36 this is specific to paragraphs as they are not considered blocks anymore
// Hint: 2-36 write `set par(spacing: ..)` instead
#show par: set block(spacing: 12pt)
