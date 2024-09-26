// Test creating a header with the query function.

--- query-here ---
// Test that `here()` yields the context element's location.
#context test(query(here()).first().func(), (context none).func())

--- query-running-header ---
#set page(
  paper: "a8",
  margin: (y: 1cm, x: 0.5cm),
  header: context {
    smallcaps[Typst Academy]
    h(1fr)
    let after = query(selector(heading).after(here()))
    let before = query(selector(heading).before(here()))
    let elem = if before.len() != 0 {
      before.last()
    } else if after.len() != 0 {
      after.first()
    }
    emph(elem.body)
  }
)

#outline()

= Introduction
#lines(1)

= Background
#lines(2)

= Approach

--- query-list-of-figures ---
#set page(
  paper: "a8",
  numbering: "1 / 1",
  margin: (bottom: 1cm, rest: 0.5cm),
)

#set figure(numbering: "I")
#show figure: set image(width: 80%)

= List of Figures
#context {
  let elements = query(selector(figure).after(here()))
  for it in elements [
    Figure
    #numbering(it.numbering,
      ..counter(figure).at(it.location())):
    #it.caption.body
    #box(width: 1fr, repeat[.])
    #counter(page).at(it.location()).first() \
  ]
}

#figure(
  image("/assets/images/cylinder.svg", width: 50%),
  caption: [Cylinder],
)

#figure(
  rect[Just some stand-in text],
  kind: image,
  supplement: "Figure",
  caption: [Stand-in text],
)

#figure(
  image("/assets/images/tetrahedron.svg", width: 50%),
  caption: [Tetrahedron],
)

--- query-complex ---
= A
== B
#figure([Cat], kind: "cat", supplement: [Other])
#heading(level: 3, outlined: false)[D]
= E <first>
#figure([Frog], kind: "frog", supplement: none)
#figure([Giraffe], kind: "giraffe", supplement: none) <second>
#figure([GiraffeCat], kind: "cat", supplement: [Other]) <second>
= H
#figure([Iguana], kind: "iguana", supplement: none)
== I

#let test-selector(selector, ref) = context {
  test(query(selector).map(e => e.body), ref)
}

// Test `or`.
#test-selector(
  heading.where(level: 1).or(heading.where(level: 3)),
  ([A], [D], [E], [H]),
)

#test-selector(
  selector(heading).and(heading.where(outlined: false)),
  ([D],)
)

#test-selector(
  heading.where(level: 1).or(
    heading.where(level: 3),
    figure.where(kind: "frog"),
  ),
  ([A], [D], [E], [Frog], [H]),
)

#test-selector(
  heading.where(level: 1).or(
    heading.where(level: 2),
    figure.where(kind: "frog"),
    figure.where(kind: "cat"),
  ),
  ([A], [B], [Cat], [E], [Frog], [GiraffeCat], [H], [I]),
)

#test-selector(
  figure.where(kind: "dog").or(heading.where(level: 3)),
  ([D],),
)

#test-selector(
  figure.where(kind: "dog").or(figure.where(kind: "fish")),
  (),
)

// Test `or` duplicates removal.
#test-selector(
  heading.where(level: 1).or(heading.where(level: 1)),
  ([A], [E], [H]),
)

// Test `and`.
#test-selector(
  figure.where(kind: "cat").and(figure.where(kind: "frog")),
  (),
)

// Test `or` with `before`/`after`
#test-selector(
  selector(heading)
    .before(<first>)
    .or(selector(figure).before(<first>)),
  ([A], [B], [Cat], [D], [E]),
)

#test-selector(
  heading.where(level: 2)
    .after(<first>)
    .or(selector(figure).after(<first>)),
  ([Frog], [Giraffe], [GiraffeCat], [Iguana], [I]),
)

// Test `and` with `after`
#test-selector(
   figure.where(kind: "cat")
    .and(figure.where(supplement: [Other]))
    .after(<first>),
   ([GiraffeCat],),
)

// Test `and` (with nested `or`)
#test-selector(
  heading.where(level: 2)
    .or(heading.where(level: 3))
    .and(heading.where(level: 2).or(heading.where(level: 1))),
  ([B], [I]),
)

#test-selector(
  heading.where(level: 2)
    .or(heading.where(level: 3), heading.where(level:1))
    .and(
      heading.where(level: 2).or(heading.where(level: 1)),
      heading.where(level: 3).or(heading.where(level: 1)),
    ),
  ([A], [E], [H]),
)

// Test `and` with `or` and `before`/`after`
#test-selector(
  heading.where(level: 1).before(<first>)
    .or(heading.where(level: 3).before(<first>))
    .and(
      heading.where(level: 1).before(<first>)
        .or(heading.where(level: 2).before(<first>))
    ),
  ([A], [E]),
)

#test-selector(
  heading.where(level: 1).before(<first>, inclusive: false)
    .or(selector(figure).after(<first>))
    .and(figure.where(kind: "iguana").or(
      figure.where(kind: "frog"),
      figure.where(kind: "cat"),
      heading.where(level: 1).after(<first>),
    )),
  ([Frog], [GiraffeCat], [Iguana])
)

--- issue-3726-query-show-set ---
// Test that show rules apply to queried elements, i.e. that the content
// returned from `query` isn't yet marked as prepared.
#set heading(numbering: "1.")
#show heading: underline
= Hi

#set heading(numbering: "I.")
#show heading: set text(blue)
#show heading: highlight.with(fill: aqua.lighten(50%))
= Bye

// New show rules apply to this, but its location and the materialized fields
// from the original are retained.
#context query(heading).join()

--- query-quote ---
// Test quoting a query.

#quote[ABC] & #quote[EFG]

#context query(selector(quote).before(here())).first()

#quote(block: true)[HIJ]
#quote(block: true)[KLM]

#context query(selector(quote).before(here())).last()

#quote[NOP] <nop>

#context query(<nop>).first()
