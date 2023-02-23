// Test foundational functions.
// Ref: false

---
#test(type(1), "integer")
#test(type(ltr), "direction")
#test(type(10 / 3), "float")

---
#test(repr(ltr), "ltr")
#test(repr((1, 2, false, )), "(1, 2, false)")

---
// Test panic.
// Error: 7-9 panicked
#panic()

---
// Test panic.
// Error: 7-12 panicked with: 123
#panic(123)

---
// Test panic.
// Error: 7-24 panicked with: "this is wrong"
#panic("this is wrong")

---
// Test failing assertions.
// Error: 8-16 assertion failed
#assert(1 == 2)

---
// Test failing assertions.
// Error: 8-51 assertion failed: two is smaller than one
#assert(2 < 1, message: "two is smaller than one")

---
// Test failing assertions.
// Error: 9-15 expected boolean, found string
#assert("true")

---
// Test the `type` function.
#test(type(1), "integer")
#test(type(ltr), "direction")
#test(type(10 / 3), "float")

---
#eval("[_Hello" + " World!_]")

---
// Error: 7-12 expected identifier
#eval("let")

---
#show raw: it => text("IBM Plex Sans", eval("[" + it.text + "]"))

Interacting
```
#set text(blue)
Blue #move(dy: -0.15em)[🌊]
```

---
// Error: 7-17 cannot continue outside of loop
#eval("continue")

---
// Error: 7-32 cannot access file system from here
#eval("include \"../coma.typ\"")

---
// Error: 7-30 cannot access file system from here
#eval("image(\"/tiger.jpg\")")

---
// Error: 23-30 cannot access file system from here
#show raw: it => eval(it.text)

```
image("/tiger.jpg")
```

---
// Error: 23-42 cannot access file system from here
#show raw: it => eval("[" + it.text + "]")

```
#show emph: _ => image("/giraffe.jpg")
_No relative giraffe!_
```

---
// Error: 7-12 expected semicolon or line break
#eval("1 2")
