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
// Test failing assertions.
// Error: 9-15 assertion failed
#assert(1 == 2)

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
#eval("_Hello" + " World!_")

---
// Error: 7-13 expected identifier
#eval("#let")

---
#show raw: it => text("IBM Plex Sans", eval(it.text))

Interacting
```
#set text(blue)
Blue #move(dy: -0.15em)[🌊]
```

---
// Error: 7-18 cannot continue outside of loop
#eval("#continue")

---
// Error: 7-33 cannot access file system from here
#eval("#include \"../coma.typ\"")

---
// Error: 7-31 cannot access file system from here
#eval("#image(\"/tiger.jpg\")")

---
// Error: 23-30 cannot access file system from here
#show raw: it => eval(it.text)

```
#image("/tiger.jpg")
```

---
// Error: 23-30 cannot access file system from here
#show raw: it => eval(it.text)

```
#show emph: _ => image("/giraffe.jpg")
_No relative giraffe!_
```

---
// Error: 7-15 expected comma
#eval("#(1 2)")
