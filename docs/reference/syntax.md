---
description: |
   A compact reference for Typst's syntax. Learn more about the language within
   markup, math, and code mode.
---

# Syntax
Typst is a markup language. This means that you can use simple syntax to
accomplish common layout tasks. The lightweight markup syntax is complemented by
set and show rules, which let you style your document easily and automatically.
All this is backed by a tightly integrated scripting language with built-in and
user-defined functions.

## Modes
Typst has three syntactical modes: Markup, math, and code. Markup mode is the
default in a Typst document, math mode lets you write mathematical formulas, and
code mode lets you use Typst's scripting features.

You can switch to a specific mode at any point by referring to the following
table:

| New mode | Syntax                          | Example                         |
|----------|---------------------------------|---------------------------------|
| Code     | Prefix the code with `#`        | `[Number: #(1 + 2)]`            |
| Math     | Surround equation with `[$..$]` | `[$-x$ is the opposite of $x$]` |
| Markup   | Surround markup with `[[..]]`   | `{let name = [*Typst!*]}`       |

Once you have entered code mode with `#`, you don't need to use further hashes
unless you switched back to markup or math mode in between.

## Markup
Typst provides built-in markup for the most common document elements. Most of
the syntax elements are just shortcuts for a corresponding function. The table
below lists all markup that is available and links to the  best place to learn
more about their syntax and usage.

| Name               | Example                      | See                      |
| ------------------ | ---------------------------- | ------------------------ |
| Paragraph break    | Blank line                   | [`parbreak`]             |
| Strong emphasis    | `[*strong*]`                 | [`strong`]               |
| Emphasis           | `[_emphasis_]`               | [`emph`]                 |
| Raw text           | ``[`print(1)`]``             | [`raw`]                  |
| Link               | `[https://typst.app/]`       | [`link`]                 |
| Label              | `[<intro>]`                  | [`label`]                |
| Reference          | `[@intro]`                   | [`ref`]                  |
| Heading            | `[= Heading]`                | [`heading`]              |
| Bullet list        | `[- item]`                   | [`list`]                 |
| Numbered list      | `[+ item]`                   | [`enum`]                 |
| Term list          | `[/ Term: description]`      | [`terms`]                |
| Math               | `[$x^2$]`                    | [Math]($category/math)   |
| Line break         | `[\]`                        | [`linebreak`]            |
| Smart quote        | `['single' or "double"]`     | [`smartquote`]           |
| Symbol shorthand   | `[~]`, `[---]`               | [Symbols]($category/symbols/sym) |
| Code expression    | `[#rect(width: 1cm)]`        | [Scripting]($scripting/#expressions) |
| Character escape   | `[Tweet at us \#ad]`         | [Below](#escapes)        |
| Comment            | `[/* block */]`, `[// line]` | [Below](#comments)       |

## Math mode { #math }
Math mode is a special markup mode that is used to typeset mathematical
formulas. It is entered by wrapping an equation in `[$]` characters. This works
both in markup and code. The equation will be typeset into its own block if it
starts and ends with at least one space (e.g. `[$ x^2 $]`). Inline math can be
produced by omitting the whitespace (e.g. `[$x^2$]`). An overview over the
syntax specific to math mode follows:

| Name                   | Example                  | See                      |
| ---------------------- | ------------------------ | ------------------------ |
| Inline math            | `[$x^2$]`                | [Math]($category/math)   |
| Block-level math       | `[$ x^2 $]`              | [Math]($category/math)   |
| Bottom attachment      | `[$x_1$]`                | [`attach`]($category/math/attach) |
| Top attachment         | `[$x^2$]`                | [`attach`]($category/math/attach) |
| Fraction               | `[$1 + (a+b)/5$]`        | [`frac`]($math.frac)     |
| Line break             | `[$x \ y$]`              | [`linebreak`]            |
| Alignment point        | `[$x &= 2 \ &= 3$]`      | [Math]($category/math)   |
| Variable access        | `[$#x$, $pi$]`           | [Math]($category/math)   |
| Field access           | `[$arrow.r.long$]`       | [Scripting]($scripting/#fields) |
| Implied multiplication | `[$x y$]`                | [Math]($category/math)   |
| Symbol shorthand       | `[$->$]`, `[$!=$]`       | [Symbols]($category/symbols/sym) |
| Text/string in math    | `[$a "is natural"$]`     | [Math]($category/math)   |
| Math function call     | `[$floor(x)$]`           | [Math]($category/math)   |
| Code expression        | `[$#rect(width: 1cm)$]`  | [Scripting]($scripting/#expressions) |
| Character escape       | `[$x\^2$]`               | [Below](#escapes)        |
| Comment                | `[$/* comment */$]`      | [Below](#comments)       |

## Code mode { #code }
Within code blocks and expressions, new expressions can start without a leading
`#` character. Many syntactic elements are specific to expressions. Below is
a table listing all syntax that is available in code mode:

| Name                     | Example                       | See                                |
| ------------------------ | ----------------------------- | ---------------------------------- |
| None                     | `{none}`                      | [`none`]                           |
| Auto                     | `{auto}`                      | [`auto`]                           |
| Boolean                  | `{false}`, `{true}`           | [`bool`]                           |
| Integer                  | `{10}`, `{0xff}`              | [`int`]                            |
| Floating-point number    | `{3.14}`, `{1e5}`             | [`float`]                          |
| Length                   | `{2pt}`, `{3mm}`, `{1em}`, .. | [`length`]                         |
| Angle                    | `{90deg}`, `{1rad}`           | [`angle`]                          |
| Fraction                 | `{2fr}`                       | [`fraction`]                       |
| Ratio                    | `{50%}`                       | [`ratio`]                          |
| String                   | `{"hello"}`                   | [`str`]                            |
| Label                    | `{<intro>}`                   | [`label`]                          |
| Math                     | `[$x^2$]`                     | [Math]($category/math)             |
| Raw text                 | ``[`print(1)`]``              | [`raw`]                            |
| Variable access          | `{x}`                         | [Scripting]($scripting/#blocks)    |
| Code block               | `{{ let x = 1; x + 2 }}`      | [Scripting]($scripting/#blocks)    |
| Content block            | `{[*Hello*]}`                 | [Scripting]($scripting/#blocks)    |
| Parenthesized expression | `{(1 + 2)}`                   | [Scripting]($scripting/#blocks)    |
| Array                    | `{(1, 2, 3)}`                 | [Array]($array)                    |
| Dictionary               | `{(a: "hi", b: 2)}`           | [Dictionary]($dictionary)          |
| Unary operator           | `{-x}`                        | [Scripting]($scripting/#operators) |
| Binary operator          | `{x + y}`                     | [Scripting]($scripting/#operators) |
| Assignment               | `{x = 1}`                     | [Scripting]($scripting/#operators) |
| Field access             | `{x.y}`                       | [Scripting]($scripting/#fields)    |
| Method call              | `{x.flatten()}`               | [Scripting]($scripting/#methods)   |
| Function call            | `{min(x, y)}`                 | [Function]($function)              |
| Argument spreading       | `{min(..nums)}`               | [Arguments]($arguments)            |
| Unnamed function         | `{(x, y) => x + y}`           | [Function]($function)              |
| Let binding              | `{let x = 1}`                 | [Scripting]($scripting/#bindings)  |
| Named function           | `{let f(x) = 2 * x}`          | [Function]($function)              |
| Set rule                 | `{set text(14pt)}`            | [Styling]($styling/#set-rules)     |
| Set-if rule              | `{set text(..) if .. }`       | [Styling]($styling/#set-rules)     |
| Show-set rule            | `{show heading: set block(..)}` | [Styling]($styling/#show-rules)  |
| Show rule with function  | `{show raw: it => {..}}`      | [Styling]($styling/#show-rules)    |
| Show-everything rule     | `{show: template}`            | [Styling]($styling/#show-rules)    |
| Context expression       | `{context text.lang}`         | [Context]($context)                |
| Conditional              | `{if x == 1 {..} else {..}}`  | [Scripting]($scripting/#conditionals) |
| For loop                 | `{for x in (1, 2, 3) {..}}`   | [Scripting]($scripting/#loops)     |
| While loop               | `{while x < 10 {..}}`         | [Scripting]($scripting/#loops)     |
| Loop control flow        | `{break, continue}`           | [Scripting]($scripting/#loops)     |
| Return from function     | `{return x}`                  | [Function]($function)              |
| Include module           | `{include "bar.typ"}`         | [Scripting]($scripting/#modules)   |
| Import module            | `{import "bar.typ"}`          | [Scripting]($scripting/#modules)   |
| Import items from module | `{import "bar.typ": a, b, c}` | [Scripting]($scripting/#modules)   |
| Comment                  | `{/* block */}`, `{// line}`  | [Below](#comments)                 |

## Comments
Comments are ignored by Typst and will not be included in the output. This is
useful to exclude old versions or to add annotations. To comment out a single
line, start it with `//`:
```example
// our data barely supports
// this claim

We show with $p < 0.05$
that the difference is
significant.
```

Comments can also be wrapped between `/*` and `*/`. In this case, the comment
can span over multiple lines:
```example
Our study design is as follows:
/* Somebody write this up:
   - 1000 participants.
   - 2x2 data design. */
```

## Escape sequences { #escapes }
Escape sequences are used to insert special characters that are hard to type or
otherwise have special meaning in Typst. To escape a character, precede it with
a backslash. To insert any Unicode codepoint, you can write a hexadecimal escape
sequence: `[\u{1f600}]`. The same kind of escape sequences also work in
[strings]($str).

```example
I got an ice cream for
\$1.50! \u{1f600}
```
