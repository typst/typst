// Test raw blocks.

--- raw-empty ---
// Empty raw block.
Empty raw block:``.

--- raw-consecutive-single-backticks ---
// No extra space.
`A``B`

--- raw-typst-lang ---
// Typst syntax inside.
```typ #let x = 1``` \
```typ #f(1)```

--- raw-block-no-parbreaks ---
// Multiline block splits paragraphs.

Text
```rust
fn code() {}
```
Text

--- raw-more-backticks ---
// Lots of backticks inside.
````
```backticks```
````

--- raw-trimming ---
// Trimming.

// Space between "rust" and "let" is trimmed.
The keyword ```rust let```.

// Trimming depends on number backticks.
(``) \
(` untrimmed `) \
(``` trimmed` ```) \
(``` trimmed ```) \
(``` trimmed```) \

--- raw-single-backtick-lang ---
// Single ticks should not have a language.
`rust let`

--- raw-dedent-first-line ---
// First line is not dedented and leading space is still possible.
     ```   A
        B
       C
     ```

--- raw-dedent-empty-line ---
// Do not take empty lines into account when computing dedent.
```
        A

        B
```

--- raw-dedent-last-line ---
// Take last line into account when computing dedent.
```
        A

        B
    ```

--- raw-tab-size ---
#set raw(tab-size: 8)

```tsv
Year	Month	Day
2000	2	3
2001	2	1
2002	3	10
```

--- raw-syntaxes ---
#set page(width: 180pt)
#set text(6pt)
#set raw(syntaxes: "/assets/syntaxes/SExpressions.sublime-syntax")

```sexp
(defun factorial (x)
  (if (zerop x)
    ; with a comment
    1
    (* x (factorial (- x 1)))))
```


--- raw-theme ---
// Test code highlighting with custom theme.
#set page(width: 180pt)
#set text(6pt)
#set raw(theme: "/assets/themes/halcyon.tmTheme")
#show raw: it => {
  set text(fill: rgb("a2aabc"))
  rect(
    width: 100%,
    inset: (x: 4pt, y: 5pt),
    radius: 4pt,
    fill: rgb("1d2433"),
    place(right, text(luma(240), it.lang)) + it,
  )
}

```typ
= Chapter 1
#lorem(100)

#let hi = "Hello World"
#show heading: emph
```

--- raw-show-set ---
// Text show rule
#show raw: set text(font: "Roboto")
`Roboto`

--- raw-align-default ---
// Text inside raw block should be unaffected by outer alignment by default.
#set align(center)
#set page(width: 180pt)
#set text(6pt)

```py
def something(x):
  return x

a = 342395823859823958329
b = 324923
```

--- raw-align-specified ---
// Text inside raw block should follow the specified alignment.
#set page(width: 180pt)
#set text(6pt)

#align(center, raw(
  lang: "typ",
  block: true,
  align: right,
  "#let f(x) = x\n#align(center, line(length: 1em))",
))

--- raw-align-invalid ---
// Error: 17-20 expected `start`, `left`, `center`, `right`, or `end`, found top
#set raw(align: top)

--- raw-inline-multiline ---
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

--- raw-highlight-typ ---
```typ
= Chapter 1
#lorem(100)

#let hi = "Hello World"
#show heading: emph
```

--- raw-highlight-typc ---
#set page(width: auto)

```typ
#set hello()
#set hello()
#set hello.world()
#set hello.my.world()
#let foo(x) = x * 2
#show heading: func
#show module.func: func
#show module.func: it => {}
#foo(ident: ident)
#hello
#hello()
#box[]
#hello.world
#hello.world()
#hello().world()
#hello.my.world
#hello.my.world()
#hello.my().world
#hello.my().world()
#{ hello }
#{ hello() }
#{ hello.world() }
#if foo []
```

--- raw-highlight-typm ---
#set page(width: auto)
```typm
1 + 2/3
a^b
hello
hello()
box[]
hello.world
hello.world()
hello.my.world()
f_zeta(x), f_zeta(x)/1
emph(hello.my.world())
emph(hello.my().world)
emph(hello.my().world())
#hello
#hello()
#hello.world
#hello.world()
#box[]
```
--- raw-highlight-rust ---
#set page(width: auto)

```rust
/// A state machine.
#[derive(Debug)]
enum State<'a> { A(u8), B(&'a str) }

fn advance(state: State<'_>) -> State<'_> {
    unimplemented!("state machine")
}
```

--- raw-highlight-py ---
#set page(width: auto)

```py
import this

def hi():
  print("Hi!")
```

--- raw-highlight-cpp ---
#set page(width: auto)

```cpp
#include <iostream>

int main() {
  std::cout << "Hello, world!";
}
```

--- raw-highlight-html ---
#set page(width: auto)

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

--- raw-blocky ---
// Test various raw parsing edge cases.

#let empty = (
  name: "empty",
  input: ``,
  text: "",
  block: false,
)

#let empty-spaces = (
  name: "empty-spaces",
  input: ```   ```,
  text: "",
  block: false,
)

#let empty-newlines = (
  name: "empty-newlines",
  input: ```


```,
  text: "\n",
  block: true,
)

#let newlines-backtick = (
  name: "newlines-backtick",
  input: ```

`

```,
  text: "\n`\n",
  block: true,
)

#let backtick = (
  name: "backtick",
  input: ``` ` ```,
  text: "`",
  block: false,
)

#let lang-backtick = (
  name: "lang-backtick",
  input: ```js ` ```,
  lang: "js",
  text: "`",
  block: false,
)

// The language tag stops on space
#let lang-space = (
  name: "lang-space",
  input: ```js test ```,
  lang: "js",
  text: "test ",
  block: false,
)

// The language tag stops on newline
#let lang-newline = (
  name: "lang-newline",
  input: ```js
test
```,
  lang: "js",
  text: "test",
  block: true,
)

// The first line and the last line are ignored
#let blocky = (
  name: "blocky",
  input: {
```
test
```
},
  text: "test",
  block: true,
)

// A blocky raw should handle dedents
#let blocky-dedent = (
  name: "blocky-dedent",
  input: {
```
 test
 ```
  },
  text: "test",
  block: true,
)

// When there is content in the first line, it should exactly eat a whitespace char.
#let blocky-dedent-firstline = (
  name: "blocky-dedent-firstline",
  input: ``` test
  ```,
  text: "test",
  block: true,
)

// When there is content in the first line, it should exactly eat a whitespace char.
#let blocky-dedent-firstline2 = (
  name: "blocky-dedent-firstline2",
  input: ``` test
```,
  text: "test",
  block: true,
)

// The first line is not affected by dedent, and the middle lines don't consider the whitespace prefix of the first line.
#let blocky-dedent-firstline3 = (
  name: "blocky-dedent-firstline3",
  input: ``` test
     test2
  ```,
  text: "test\n   test2",
  block: true,
)

// The first line is not affected by dedent, and the middle lines don't consider the whitespace prefix of the first line.
#let blocky-dedent-firstline4 = (
  name: "blocky-dedent-firstline4",
  input: ```     test
  test2
  ```,
  text: "    test\ntest2",
  block: true,
)

#let blocky-dedent-lastline = (
  name: "blocky-dedent-lastline",
  input: ```
  test
 ```,
  text: " test",
  block: true,
)

#let blocky-dedent-lastline2 = (
  name: "blocky-dedent-lastline2",
  input: ```
  test
   ```,
  text: "test",
  block: true,
)

#let blocky-tab = (
  name: "blocky-tab",
  input: {
```
	test
```
},
  text: "\ttest",
  block: true,
)

// This one is a bit problematic because there is a trailing tab below "test"
// which the editor constantly wants to remove.
#let blocky-tab-dedent = (
  name: "blocky-tab-dedent",
  input: eval("```\n\ttest\n  \n ```"),
  text: "test\n ",
  block: true,
)

#let extra-first-line-ws = (
  name: "extra-first-line-ws",
  input: eval("```   \n```"),
  text: "",
  block: true,
)

#let cases = (
  empty,
  empty-spaces,
  empty-newlines,
  newlines-backtick,
  backtick,
  lang-backtick,
  lang-space,
  lang-newline,
  blocky,
  blocky-dedent,
  blocky-dedent-firstline,
  blocky-dedent-firstline2,
  blocky-dedent-firstline3,
  blocky-dedent-lastline,
  blocky-dedent-lastline2,
  blocky-tab,
  blocky-tab-dedent,
  extra-first-line-ws,
)

#for c in cases {
  let block = c.block
  assert.eq(c.text, c.input.text, message: "in point " + c.name + ", expect " + repr(c.text) + ", got " + repr(c.input.text) + "")
  assert.eq(block, c.input.block, message: "in point " + c.name + ", expect " + repr(block) + ", got " + repr(c.input.block) + "")
}

--- raw-line ---
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

--- raw-line-alternating-fill ---
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

--- raw-line-text-fill ---
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

--- raw-line-scripting ---

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

--- issue-3601-empty-raw ---
// Test that empty raw block with `typ` language doesn't cause a crash.
```typ
```

--- raw-empty-lines ---
// Test raw with multiple empty lines.

#show raw: block.with(width: 100%, fill: gray)

```




```

--- issue-3841-tabs-in-raw-type-code ---
// Tab chars were not rendered in raw blocks with lang: "typ(c)"
#raw("#if true {\n\tf()\t// typ\n}", lang: "typ")

#raw("if true {\n\tf()\t// typc\n}", lang: "typc")

```typ
#if true {
	// tabs around f()
	f()	// typ
}
```

```typc
if true {
	// tabs around f()
	f()	// typc
}
```

--- issue-4662-math-mode-language-for-raw ---
// Test lang: "typm" syntax highlighting without enclosing dollar signs
#raw("pi^2", lang: "typm")

--- issue-2259-raw-color-overwrite ---
// Test that the color of a raw block is not overwritten
#show raw: set text(fill: blue)

`Hello, World!`

```rs
fn main() {
    println!("Hello, World!");
}
```

--- issue-3191-raw-justify ---
// Raw blocks should not be justified by default.
```
a b c --------------------
```

#show raw: set par(justify: true)
```
a b c --------------------
```

--- issue-3191-raw-normal-paragraphs-still-shrink ---
// In normal paragraphs, spaces should still be shrunk.
// The first line here serves as a reference, while the second
// uses non-breaking spaces to create an overflowing line
// (which should shrink).
~~~~No shrinking here

~~~~The~spaces~on~this~line~shrink

--- issue-3820-raw-space-when-end-with-backtick ---
```typ
`code`
```

  ```typ
  `code`
  ```

--- issue-5760-disable-cjk-latin-spacing-in-raw ---

#show raw: set text(cjk-latin-spacing: auto)
```typ
#let hi = "你好world"
```

#show raw: set text(cjk-latin-spacing: none)
```typ
#let hi = "你好world"
```

--- raw-theme-set-to-auto ---
```typ
#let hi = "Hello World"
```

#set raw(theme: "/assets/themes/halcyon.tmTheme")
```typ
#let hi = "Hello World"
```

#set raw(theme: auto)
```typ
#let hi = "Hello World"
```

--- raw-theme-set-to-none ---
#set raw(theme: none)
```typ
#let foo = "bar"
```

--- raw-unclosed ---
// Test unterminated raw text.
//
// Note: This test should be the final one in the file because it messes up
// syntax highlighting.
//
// Error: 1-2:1 unclosed raw text
`endless
