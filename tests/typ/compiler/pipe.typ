// Test piping.
// Ref: false

#let test(x,y) = if x != y {panic()}

---
// Ref: false

#let add2(x) = x + 2
#{
  let five = 1 |> add2 |> add2
  test(five,5)
}
 
---

// Error: 1:16-1:21 cannot add function and integer
#let add2(x) = x + 2
#{
  add2 |> add2
}


---

// Error: 1:41-1:43 panicked
#let p(x) = if type(x) == type(2) {panic()}
#{2 |> p}

---
// Ref: false

// this should just be markdown
#let p(x) = if type(x) == type(2) {panic()}

#2 |> p

#2|>p

---
// Ref: false

#let sum(..x) = {
  x.pos().fold(0, (i,j) => i+j)
}

#{
 test(2 |> sum(1,1),4)
}

---
// Ref: false

//should not crash or anything
#{
  let f(..x) = []
  (2,3).. |> f
}

---
// Ref: false

#let pos(..x) = x.pos() 
#{  
test((2,4).. |> pos(1,_,3,_), pos(1,2,3,4))
}  

---
// Ref: false

//#let named(..x) = x.named()
//#{  
//  test((x : 2, y: 4).. |> named(1,_,3,_), (x : 2, y : 4))
//}  

---
// Ref: true
approve if you have two identical looking results.
#{
  [Lorem Ipsum] |> text(size:14pt) |> align(left) |> box(stroke : 1mm, inset : 20pt)

  box(stroke: 1mm, inset : 20pt, align(left,text(size : 14pt, [Lorem Ipsum])))
}

