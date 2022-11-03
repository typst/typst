// Test the `eval` function.

---
#eval("_Hello" + " World!_")

---
// Error: 7-13 expected identifier
#eval("#let")

---
#set raw(around: none)
#show it: raw as text("IBM Plex Sans", eval(it.text))

Interacting
```
#set text(blue)
Blue #move(dy: -0.15em)[🌊]
```

---
// Error: 7-19 cannot continue outside of loop
#eval("{continue}")

---
// Error: 7-33 cannot access file system from here
#eval("#include \"../coma.typ\"")

---
// Error: 7-35 cannot access file system from here
#eval("#image(\"/res/tiger.jpg\")")

---
// Error: 23-30 cannot access file system from here
#show it: raw as eval(it.text)

```
#show strong as image("/res/tiger.jpg")
*No absolute tiger!*
```

---
// Error: 23-30 cannot access file system from here
#show it: raw as eval(it.text)

```
#show emph as image("../../res/giraffe.jpg")
_No relative giraffe!_
```

---
// Error: 7-16 expected comma
#eval("{(1 2)}")
