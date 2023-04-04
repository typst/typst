// Test piping.
// Ref: false

---

#let add2(x) = x + 2
#{
  let five = 1 |> add2 |> add2
  test(five,5)
}
 
---
#let add2(x) = x + 2
#{
// cannot add function and integer
  add2 |> add2
}


---
// Error: 32:41-32:43: panicked
#let p(x) = if type(x) == type(2) {panic()}

#{2 |> p}

---
// this should just be markdown
#let p(x) = if type(x) == type(2) {panic()}

#2 |> p

#2|>p
---
// Ref: false

[Lorem Ipsum] |> text.with(size:14pt) |> align.with(center) |> box.with(stroke : 1mm, inset : 10pt) |> align.with(right)


//#test(2, (1,1) |> f )
