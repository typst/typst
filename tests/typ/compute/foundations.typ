// Test foundational functions.
// Ref: false

---
#test(type(1), int)
#test(type(ltr), direction)
#test(type(10 / 3), float)

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
// Test failing assertions.
// Error: 11-19 equality assertion failed: value 10 was not equal to 11
#assert.eq(10, 11)

---
// Test failing assertions.
// Error: 11-55 equality assertion failed: 10 and 12 are not equal
#assert.eq(10, 12, message: "10 and 12 are not equal")

---
// Test failing assertions.
// Error: 11-19 inequality assertion failed: value 11 was equal to 11
#assert.ne(11, 11)

---
// Test failing assertions.
// Error: 11-57 inequality assertion failed: must be different from 11
#assert.ne(11, 11, message: "must be different from 11")

---
// Test successful assertions.
#assert(5 > 3)
#assert.eq(15, 15)
#assert.ne(10, 12)

---
// Test the `type` function.
#test(type(1), int)
#test(type(ltr), direction)
#test(type(10 / 3), float)

---
// Test the eval function.
#test(eval("1 + 2"), 3)
#test(eval("1 + x", scope: (x: 3)), 4)
#test(eval("let x = x + 1; x + 1", scope: (x: 1)), 3)

---
// Test evaluation in other modes.
// Ref: true
#eval("[_Hello" + " World!_]") \
#eval("_Hello" + " World!_", mode: "markup") \
#eval("RR_1^NN", mode: "math", scope: (RR: math.NN, NN: math.RR))

---
// Error: 7-12 expected identifier
#eval("let")

---
#show raw: it => text(font: "PT Sans", eval("[" + it.text + "]"))

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
