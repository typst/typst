[font 8pt]

// Typst syntax inside.
`#let x = 1``[f 1]`

// Space between "rust" and "let" is trimmed.
The keyword ``rust let``.

// Trimming depends on number backticks.
<` untrimmed `> \
<`` trimmed ``>

// Multiline trimming.
``py
import this

def say_hi():
    print("Hi!")
``

// Lots of backticks inside.
````
```backticks```
````

// Unterminated.
// Error: 2:1-2:1 expected backtick(s)
`endless
