// Test line in raw code.

---
#set page(width: 200pt)

```rs
fn main() {
    println!("Hello, world!");
}
```

#show raw.line: it => {
  box(stack(
    dir: ltr,
    box(width: 15pt)[#it.number],
    it.body,
  ))
  linebreak()
}

```rs
fn main() {
    println!("Hello, world!");
}
```

---
#set page(width: 200pt)
#show raw: it => stack(dir: ttb, ..it.lines)
#show raw.line: it => {
  box(
    width: 100%,
    height: 1.75em,
    inset: 0.25em,
    fill: if calc.rem(it.number, 2) == 0 {
      luma(90%)
    } else {
      white
    },
    align(horizon, stack(
      dir: ltr,
      box(width: 15pt)[#it.number],
      it.body,
    ))
  )
}

```typ
#show raw.line: block.with(
  fill: luma(60%)
);

Hello, world!

= A heading for good measure
```

---
#set page(width: 200pt)
#show raw.line: set text(fill: red)

```py
import numpy as np

def f(x):
    return x**2

x = np.linspace(0, 10, 100)
y = f(x)

print(x)
print(y)
```

---
// Ref: false

// Test line extraction works.

#show raw: code => {
  for i in code.lines {
    test(i.count, 10)
  }

  test(code.lines.at(0).text, "import numpy as np")
  test(code.lines.at(1).text, "")
  test(code.lines.at(2).text, "def f(x):")
  test(code.lines.at(3).text, "    return x**2")
  test(code.lines.at(4).text, "")
  test(code.lines.at(5).text, "x = np.linspace(0, 10, 100)")
  test(code.lines.at(6).text, "y = f(x)")
  test(code.lines.at(7).text, "")
  test(code.lines.at(8).text, "print(x)")
  test(code.lines.at(9).text, "print(y)")
  test(code.lines.at(10, default: none), none)
}

```py
import numpy as np

def f(x):
    return x**2

x = np.linspace(0, 10, 100)
y = f(x)

print(x)
print(y)
```
