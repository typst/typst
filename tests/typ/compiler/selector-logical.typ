//Tests for logical (and/or) selectors

---
= A
== B
#figure([Cat], kind: "cat", supplement: [Other])
=== D
= E <first>
#figure([Frog], kind: "frog", supplement: none)
#figure([Giraffe], kind: "giraffe", supplement: none) <second>
#figure([GiraffeCat], kind: "cat", supplement: [Other]) <second>
= H
#figure([Iguana], kind: "iguana", supplement: none)
== I

#let test-selector(selector, ref) = locate(loc => {
  let elems = query(selector, loc)
  test(elems.map(e => e.body), ref)
})

// Test `or`.
#test-selector(
  heading.where(level: 1).or(heading.where(level: 3)),
  ([A], [D], [E], [H]),
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
