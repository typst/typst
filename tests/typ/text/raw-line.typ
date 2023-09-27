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
        box(width: 15pt)[#it.line],
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
#show raw.line: it => {
    box(stack(
        dir: ltr,
        box(width: 15pt)[#it.line],
        it.body,
    ), fill: if calc.rem(it.line, 2) == 0 { luma(40%) } else { white })
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
#show raw: it => block[
    #stack(dir: ttb, spacing: 0pt, ..it.lines)
    #place(top + right, dx: -0.5em, dy: 0.25em, box(inset: 0.5em, fill: orange.lighten(80%), radius: 0.25em)[ #it.lang ])
]
#show raw.line: it => box(
    fill: if calc.rem(it.line, 2) == 0 { luma(80%) } else { white },
    radius: if it.first and it.last {
        0.25em
    } else if it.first {
        (top-left: 0.25em, top-right: 0.25em)
    } else if it.last {
        (bottom-left: 0.25em, bottom-right: 0.25em)
    } else {
        0pt
    },
    stroke: if it.first and it.last {
        0.5pt + luma(200)
    } else if it.first {
        (top: 1.5pt + luma(200), x: 1.5pt + luma(200))
    } else if it.last {
        (bottom: 1.5pt + luma(200), x: 1.5pt + luma(200))
    } else {
        (left: 1.5pt + luma(200), right: 1.5pt + luma(200))
    },
    width: 100%,
    inset: 0.75em,
    stack(
        dir: ltr,
        box(width: 15pt)[#it.line],
        it.body,
    )
)

```py
import numpy as np

def f(x):
    return x**2

x = np.linspace(0, 10, 100)
y = f(x)

print(x)
print(y)
```