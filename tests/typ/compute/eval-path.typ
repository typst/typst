// Test file loading in eval.

---
// Test absolute path.
#eval("image(\"/files/tiger.jpg\", width: 50%)")

---
#show raw: it => eval(it.text, mode: "markup")

```
#show emph: image("/files/tiger.jpg", width: 50%)
_Tiger!_
```

---
// Test relative path.
// Ref: false
#test(eval(`"HELLO" in read("./eval-path.typ")`.text), true)
