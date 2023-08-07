//Tests for logical (and/or) selectors

---
// Test `or`

= A
== B
#figure([Cat], kind: "cat", supplement: none)
=== D
= E
#figure([Frog], kind: "frog", supplement: none)
== G
== H

#let refr = ([A], [D], [E])
#locate(loc => {
    let q = query(heading.where(level: 1)
        .or(heading.where(level: 3)),
    loc);
    let q = q.map(e => { e.body } )
    test(q, refr)
}) 

#let refr = ([A], [D], [E], [Frog])
#locate(loc => {
    let q = query(heading.where(level: 1)
        .or(heading.where(level: 3))
        .or(figure.where(kind: "frog")),
    loc);
    let q = q.map( e => e.body)
    test(q, refr)
}) 

#let refr = ([A], [B], [Cat], [E], [Frog], [G], [H])
#locate(loc => {
    let q = query(heading.where(level: 1)
        .or(heading.where(level: 2))
        .or(figure.where(kind: "frog"))
        .or(figure.where(kind: "cat")),
    loc);
    test(q.map(e => e.body), refr)
}) 

---
// Test empty matches

= A
== B
#figure([Cat], kind: "cat", supplement: none)
=== D
= E
#figure([Frog], kind: "frog", supplement: none)
== G
== H

#let refr = ([D],)
#locate(loc => {
    let q = query(figure.where(kind: "dog")
        .or(heading.where(level: 3)),
    loc);
    let q = q.map(e => { e.body } )
    test(q, refr)
}) 

#locate(loc => {
    let q = query(figure.where(kind: "dog")
        .or(figure.where(kind: "fish")),
    loc);
    test(q.len(), 0)
}) 

#locate(loc => {
    let q = query(figure.where(kind: "dog")
        .and(figure.where(kind: "frog")),
    loc);
    test(q.len(), 0)
}) 

---
// Test `or` duplicates removal
= A
= B
== C <C>
= D
== E

#let refr = ([A], [B], [D])
#locate(loc => {
    let q = query(heading.where(level: 1).or(heading.where(level: 1)), loc)
    let q = q.map(e => { e.body } )
    test(q, refr)
}) 

---
// Test `or` with `before`/`after`

= A
== B
#figure([Cat], kind: "cat", supplement: none)
=== D
= E
<label>
#figure([Frog], kind: "frog", supplement: none)
#figure([Giraffe], kind: "giraffe", supplement: none)
= H
== I

#let refr = ([A], [B], [Cat], [D], [E])
#locate(loc => {
    let q = query(selector(heading).before(<label>)
        .or(selector(figure).before(<label>)),
    loc);
    let q = q.map(e => { e.body } )
    test(q, refr)
}) 

#let refr = ([Frog], [Giraffe], [I])
#locate(loc => {
    let q = query(heading.where(level: 2).after(<label>)
        .or(selector(figure).after(<label>)),
    loc);
    let q = q.map( e => e.body)
    test(q, refr)
}) 

---
// Test `and` with `after`

= A
== B
#figure([Cat], kind: "cat", supplement: [Other])
=== D
= E
<first>
#figure([Frog], kind: "frog", supplement: none)
#figure([GiraffeCat], kind: "cat", supplement: [Other])
<second>
= H
== I

#let refr = ([GiraffeCat],)
#locate(loc => {
    let q = query(figure.where(kind: "cat").and(
        figure.where(supplement: [Other])).after(<first>),
        loc)
    let q = q.map(e => { e.body } )
    test(q, refr)
}) 

---
// Test `and` (with nested `or`)

= A
== B
#figure([Cat], kind: "cat", supplement: none)
=== D
= E
#figure([Frog], kind: "frog", supplement: none)
#figure([Giraffe], kind: "giraffe", supplement: none)
= H
== I

#let refr = ([B], [I])
#locate(loc => {
    let q = query(
        (heading.where(level: 2).or( heading.where(level: 3)))
        .and(
          heading.where(level: 2).or( heading.where(level: 1)))
         , loc)
    let q = q.map(e => { e.body } )
    test(q, refr)
}) 

#let refr = ([A], [E], [H])
#locate(loc => {
    let q = query(
        (heading.where(level: 2).or( heading.where(level: 3))).or(heading.where(level:1))
        .and(
          heading.where(level: 2).or( heading.where(level: 1))
        )
        .and(
          heading.where(level: 3).or( heading.where(level: 1))
        ), loc)
    let q = q.map(e => { e.body } )
    test(q, refr)
}) 

---
// Test `and` with `or` and `before`/`after`

= A
== B
#figure([Cat], kind: "cat", supplement: none)
=== D
= E
#let refr = ([A], [E])
#locate(loc => {
    let q = query(
            heading.where(level: 1).before(loc)
        .or(
            heading.where(level: 3).before(loc))
    .and(
            heading.where(level: 1).before(loc)
        .or(
            heading.where(level: 2).before(loc))), loc)
    let q = q.map(e => { e.body } )
    test(q, refr)
})
#let refr = ([Frog], [Iguana])
#locate(loc => {
    let q = query(
            heading.where(level: 1).before(loc)
        .or(
            selector(figure).after(loc))
    .and(
            figure.where(kind: "iguana")
        .or(
            figure.where(kind: "frog"))
        .or(
            figure.where(kind: "cat"))
        .or(
            heading.where(level: 1).after(loc))),
    loc)
    let q = q.map(e => { e.body } )
    test(q, refr)
})
#figure([Frog], kind: "frog", supplement: none)
#figure([Giraffe], kind: "giraffe", supplement: none)
= H
#figure([Iguana], kind: "iguana", supplement: none)
== J
