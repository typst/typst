// Test raw blocks.

---
// No extra space.
`A``B`

---
// Empty raw block.
Empty raw block:``.

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
(``) \
(` untrimmed `) \
(``` trimmed` ```) \
(``` trimmed ```) \
(``` trimmed```) \

---
// Single ticks should not have a language.
`rust let`

---
// First line is not dedented and leading space is still possible.
     ```   A
        B
       C
     ```

---
// Do not take empty lines into account when computing dedent.
```
        A

        B
```

---
// Take last line into account when computing dedent.
```
        A

        B
    ```

---
// Text show rule
#show raw: set text(font: "Roboto")
`Roboto`

---
// Unterminated.
// Error: 1-2:1 unclosed raw text
`endless
