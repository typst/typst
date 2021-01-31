// Test raw blocks.

---
// No extra space.
`A``B`

---
// Typst syntax inside.
`#let x = 1` \
`#[f 1]`

---
// Trimming.

// Space between "rust" and "let" is trimmed.
The keyword ```rust let```.

// Trimming depends on number backticks.
<``> \
<` untrimmed `> \
<``` trimmed ```>

// Multiline trimming.
```py
import this

def hi():
  print("Hi!")
```

---
// Lots of backticks inside.
````
```backticks```
````

---
// Unterminated.
// Error: 2:1-2:1 expected backtick(s)
`endless
