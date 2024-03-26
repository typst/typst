// Test code highlighting.

---
#set page(width: 180pt)
#set text(6pt)
#show raw: it => rect(
  width: 100%,
  inset: (x: 4pt, y: 5pt),
  radius: 4pt,
  fill: rgb(239, 241, 243),
  place(right, text(luma(110), it.lang)) + it,
)

```typ
= Chapter 1
#lorem(100)

#let hi = "Hello World"
#show heading: emph
```

```rust
/// A carefully designed state machine.
#[derive(Debug)]
enum State<'a> { A(u8), B(&'a str) }

fn advance(state: State<'_>) -> State<'_> {
    unimplemented!("state machine")
}
```

```py
import this

def hi():
  print("Hi!")
```

```cpp
#include <iostream>

int main() {
  std::cout << "Hello, world!";
}
```

```julia
# Add two numbers
function add(x, y)
    return x * y
end
```

    // Try with some indent.
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

---
#set page(width: 180pt)
#set text(6pt)
#set raw(lang:"python")

Inline raws, multiline e.g. `for i in range(10):
  # Only this line is a comment.
  print(i)` or otherwise e.g. `print(j)`, are colored properly.

Inline raws, multiline e.g. `
# Appears blocky due to linebreaks at the boundary.
for i in range(10):
  print(i)
` or otherwise e.g. `print(j)`, are colored properly.
