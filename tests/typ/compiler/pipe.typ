// Test piping.
// Ref: false

#let test(x,y) = if x != y {panic()}

---

#let add2(x) = x + 2
#{
  let five = 1 |> add2 |> add2
  test(five,5)
}
 
---

// Error: 1:16-1:21 cannot add function and integer
#let add2(x) = x + 2
#{
//  add2 |> add2
}


---

// Error: 1:41-1:43 panicked
#let p(x) = if type(x) == type(2) {panic()}
//#{2 |> p}

---

// this should just be markdown
#let p(x) = if type(x) == type(2) {panic()}

#2 |> p

#2|>p

---

#let sum(..x) = {
  x.pos().fold(0, (i,j) => i+j)
}

#{
 test(2 |> sum(1,1),4)
}

---

// Subset 6
#let pos(..x) = x.pos()
#{
  test( 2 |> pos(1,_,3),(1,2,3))
}

---

#let pos(..x) = x.pos()

#{
  let x = (2,4).. |> pos(1,_,3,_) 
  let y = (1,2,3,4)
  test(x, y)
}

---

//should not crash or anything
#{
  let f(..x) = []
  (2,3).. |> f
}

---
// Ref: false
#{
[Lorem Ipsum] |> text(size:14pt) |> align(center) |> box(stroke : 1mm, inset : 10pt) |> align(right)
}


//#test(2, (1,1) |> f )
