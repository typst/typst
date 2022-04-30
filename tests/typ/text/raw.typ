// Test raw blocks.

---
// No extra space.
`A``B`

---
// Typst syntax inside.
```typ #let x = 1``` \
```typ #f(1)```

---
// Multiline block splits paragraphs.

Text
```rust
fn code() {}
```
Text

---
// Lots of backticks inside.
````
```backticks```
````

---
// Trimming.

// Space between "rust" and "let" is trimmed.
The keyword ```rust let```.

// Trimming depends on number backticks.
<``> \
<` untrimmed `> \
<``` trimmed` ```> \
<``` trimmed ```> \
<``` trimmed```>

// Multiline trimming and dedenting.
#block[
  ```py
  import this

  def hi():
    print("Hi!")
  ```
]

---
// First line is not dedented and leading space is still possible.
     ```   A
        B
       C```

---
// Unterminated.
// Error: 2:1 expected 1 backtick
`endless
