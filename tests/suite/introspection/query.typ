// Test creating a header with the query function.

--- query-here paged empty ---
// Test that `here()` yields the context element's location.
#context test(query(here()).first().func(), (context none).func())

--- query-running-header paged ---
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

--- query-list-of-figures paged ---
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
    #counter(figure).display(at: it.location()):
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

--- query-complex paged ---
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

--- query-within paged html bundle ---
// The within selector is not yet publicly exposed, but already used internally,
// so it's good to have some tests. Since it's not yet in the public API, we
// have a test-runner specific `selector-within` "polyfill" instead.

// We also want to test in bundle mode to ensure the inner introspector
// correctly forwards the stuff.
#show: it => context if target() == "bundle" {
  document("main.pdf", it)
} else {
  it
}

#let test-selector(selector, ref) = context {
  test(query(selector).map(e => e.body), ref)
}

= #emph[Hi *there*]

What's *up* with *you?*

#figure([Empty], caption: [A *nice* *rect*])

// Test that the within query gracefully handles a case where an insertion
// immediately precedes the end tag.
#quote(footnote[_Hello_])

#context [
  #let loc = here()
  *Local* bold *text*
  #test-selector(
    selector-within(strong, loc),
    ([Local], [text]),
  )
]

#test-selector(
  selector-within(strong, par),
  ([up], [you?], [Local], [text]),
)

#test-selector(
  selector-within(strong, selector.or(heading, emph, figure)),
  ([there], [nice], [rect]),
)

#test-selector(
  selector-within(selector-within(strong, emph), heading),
  ([there],),
)

#test-selector(
  selector-within(selector-within(strong, heading), emph),
  ([there],),
)

#test-selector(
  selector-within(strong, selector-within(heading, emph)),
  (),
)

#test-selector(
  selector-within(emph, quote),
  ([Hello],),
)

--- query-within-document bundle ---
#let test-selector(selector, ref) = context {
  test(query(selector).map(e => e.body), ref)
}

#document("a.html")[
  = 1
  #table[
    = 2
  ][
    = 3
  ]
] <a>

#document("b.html")[
  = 4
  = 5
] <b>

#test-selector(selector-within(heading, table), ([2], [3]))
#test-selector(selector-within(heading, <a>), ([1], [2], [3]))
#test-selector(selector-within(heading, <b>), ([4], [5]))

--- query-bundle-logical-order bundle ---
#let m(s) = [#metadata(s) <hi>]

#metadata(1)
#document("hi.html")[
  #metadata("a")
  HTML
]
#metadata(2)
#asset("data.json", "data")
#metadata(3)
#document("hi.pdf")[
  #metadata("b")
  PDF
]
#metadata(4)

// Test logical order across documents.
#context {
  test(query(metadata).map(v => v.value), (1, "a", 2, 3, "b", 4))
}

// Test querying documents and assets.
#context {
  test(query(document).map(v => v.path), ("/hi.html", "/hi.pdf"))
  test(query(asset).map(v => v.path), ("/data.json",))
  test(
    query(selector.or(document, asset)).map(v => v.path),
    ("/hi.html", "/data.json", "/hi.pdf"),
  )
  test(
    query(selector(document).after(metadata.where(value: 3))).map(v => v.path),
    ("/hi.pdf",),
  )
}

--- query-bundle-logical-order-around-html bundle ---
#metadata(1)
#document("hi.html")[
  // Test tags around the HTML root element.
  #metadata("a")
  #html.html[
    #metadata("b")
    #html.body[Hello World!]
    #metadata("c")
  ]
  #metadata("d")
]
#metadata(2)

#context test(
  query(metadata).map(v => v.value),
  (1, "a", "b", "c", "d", 2)
)

--- issue-3726-query-show-set paged ---
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

--- query-quote paged ---
// Test quoting a query.

#quote[ABC] & #quote[EFG]

#context query(selector(quote).before(here())).first()

#quote(block: true)[HIJ]
#quote(block: true)[KLM]

#context query(selector(quote).before(here())).last()

#quote[NOP] <nop>

#context query(<nop>).first()

--- issue-5117-query-order-place paged empty ---
#let t(expected) = context {
  let elems = query(selector(metadata).after(here()))
  let val = elems.first().value
  test(val, expected)
}

#{
  t("a")
  place(metadata("a"))
}

#{
  t("b")
  block(height: 1fr, metadata("b"))
}
