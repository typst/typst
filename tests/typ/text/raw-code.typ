// Test code highlighting.

---
#set page(width: 180pt)
#set text(6pt)
```typ
= Chapter 1
#lorem(100)

#let hi = "Hello World"
#show heading: emph
```

---
#set page(width: 180pt)
#set text(6pt)

```rust
/// A carefully designed state machine.
#[derive(Debug)]
enum State<'a> { A(u8), B(&'a str) }

fn advance(state: State<'_>) -> State<'_> {
    unimplemented!("state machine")
}
```

---
#set page(width: 180pt)
#set text(6pt)

```py
import this

def hi():
  print("Hi!")
```

---
#set page(width: 180pt)
#set text(6pt)

```cpp
#include <iostream>

int main() {
  std::cout << "Hello, world!";
}
```

---
#set page(width: 180pt)
#set text(6pt)

#rect(inset: (x: 4pt, y: 5pt), radius: 4pt, fill: rgb(239, 241, 243))[
  ```html
  <!DOCTYPE html>
  <html>
    <head>
      <meta charset="utf-8">
    </head>
    <body>
      <h1>Topic</h1>
      <p>The Hypertext Markup Language.</p>
      <script>
        function foo(a, b) {
          return a + b + "string";
        }
      </script>
    </body>
  </html>
  ```
]
