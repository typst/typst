#set page(width: auto)
#set text("Latin Modern Roman")
#show <table>: it => table(
  columns: 2,
  inset: 8pt,
  ..it.text
    .split("\n")
    .map(line => (text(10pt, raw(line, lang: "typ")), eval(line) + [ ]))
    .flatten()
)

```
Let $x in NN$ be ...
$ (1 + x/2)^2 $
$ x arrow:l y $
$ sum_(n=1)^mu 1 + (2pi (5 + n)) / k $
$ { x in RR | x "is natural" and x < 10 } $
$ sqrt(x^2) = frac(x, 1) $
$ "profit" = "income" - "expenses" $
$ x < #for i in range(5) [$ #i < $] y $
$ 1 + 2 = #{1 + 2} $
$ A sub:eq:not B $
```
<table>

---
// Error: 8 expected closing paren
$ sum_( $
