#import "../../../components/index.typ": docs-chapter, docs-table

#show: docs-chapter.with(
  title: "Syntax",
  route: "/reference/language/syntax",
  description: "A compact reference for Typst's syntax. Learn more about the language within markup, math, and code mode.",
)

Typst is a markup language. This means that you can use simple syntax to accomplish common layout tasks. The lightweight markup syntax is complemented by set and show rules, which let you style your document easily and automatically. All this is backed by a tightly integrated scripting language with built-in and user-defined functions.

= Modes <modes>
Typst has three syntactical modes: Markup, math, and code. Markup mode is the default in a Typst document, math mode lets you write mathematical formulas, and code mode lets you use Typst's scripting features.

You can switch to a specific mode at any point by referring to the following table:

#docs-table(
  table.header[New mode][Syntax][Example],

  [Code],
  [Prefix the code with `#`],
  [`[Number: #(1 + 2)]`],

  [Math],
  [Surround equation with `[$..$]`],
  [`[$-x$ is the opposite of $x$]`],

  [Markup],
  [Surround markup with `[[..]]`],
  [`{let name = [*Typst!*]}`],
)

Once you have entered code mode with `#`, you don't need to use further hashes unless you switched back to markup or math mode in between.

= Markup <markup>
Typst provides built-in markup for the most common document elements. Most of the syntax elements are just shortcuts for a corresponding function. The table below lists all markup that is available and links to the  best place to learn more about their syntax and usage.

#docs-table(
  table.header[Name][Example][See],

  [Paragraph break],
  [Blank line],
  [@parbreak],

  [Strong emphasis],
  [`[*strong*]`],
  [@strong],

  [Emphasis],
  [`[_emphasis_]`],
  [@emph],

  [Raw text],
  [``` [`print(1)`]```],
  [@raw],

  [Link],
  [`[https://typst.app/]`],
  [@link],

  [Label],
  [`[<intro>]`],
  [@label],

  [Reference],
  [`[@intro]`],
  [@ref],

  [Heading],
  [`[= Heading]`],
  [@heading],

  [Bullet list],
  [`[- item]`],
  [@list],

  [Numbered list],
  [`[+ item]`],
  [@enum],

  [Term list],
  [`[/ Term: description]`],
  [@terms],

  [Math],
  [`[$x^2$]`],
  [@math[Math]],

  [Line break],
  [`[\]`],
  [@linebreak],

  [Smart quote],
  [`['single' or "double"]`],
  [@smartquote],

  [Symbol shorthand],
  [`[~]`, `[---]`],
  [@reference:symbols:shorthands[Symbols]],

  [Code expression],
  [`[#rect(width: 1cm)]`],
  [@reference:scripting:expressions[Scripting]],

  [Character escape],
  [`[Tweet at us \#ad]`],
  [@reference:syntax:escapes[Below]],

  [Comment],
  [`[/* block */]`, `[// line]`],
  [@reference:syntax:comments[Below]],
)

= Math mode <math>
Math mode is a special markup mode that is used to typeset mathematical formulas. It is entered by wrapping an equation in `[$]` characters. This works both in markup and code. The equation will be typeset into its own block if it starts and ends with at least one space (e.g. `[$ x^2 $]`). Inline math can be produced by omitting the whitespace (e.g. `[$x^2$]`). An overview over the syntax specific to math mode follows:

#docs-table(
  table.header[Name][Example][See],

  [Inline math],
  [`[$x^2$]`],
  [@math[Math]],

  [Block-level math],
  [`[$ x^2 $]`],
  [@math[Math]],

  [Bottom attachment],
  [`[$x_1$]`],
  [@math:attach[`attach`]],

  [Top attachment],
  [`[$x^2$]`],
  [@math:attach[`attach`]],

  [Fraction],
  [`[$1 + (a+b)/5$]`],
  [@math.frac[`frac`]],

  [Line break],
  [`[$x \ y$]`],
  [@linebreak],

  [Alignment point],
  [`[$x &= 2 \ &= 3$]`],
  [@math[Math]],

  [Variable access],
  [`[$#x$, $pi$]`],
  [@math[Math]],

  [Field access],
  [`[$arrow.r.long$]`],
  [@reference:scripting:fields[Scripting]],

  [Implied multiplication],
  [`[$x y$]`],
  [@math[Math]],

  [Symbol shorthand],
  [`[$->$]`, `[$!=$]`],
  [@reference:symbols:shorthands[Symbols]],

  [Text/string in math],
  [`[$a "is natural"$]`],
  [@math[Math]],

  [Math function call],
  [`[$floor(x)$]`],
  [@math[Math]],

  [Code expression],
  [`[$#rect(width: 1cm)$]`],
  [@reference:scripting:expressions[Scripting]],

  [Character escape],
  [`[$x\^2$]`],
  [@reference:syntax:escapes[Below]],

  [Comment],
  [`[$/* comment */$]`],
  [@reference:syntax:comments[Below]],
)

= Code mode <code>
Within code blocks and expressions, new expressions can start without a leading `#` character. Many syntactic elements are specific to expressions. Below is a table listing all syntax that is available in code mode:

#docs-table(
  table.header[Name][Example][See],

  [None],
  [`{none}`],
  [@none],

  [Auto],
  [`{auto}`],
  [@auto],

  [Boolean],
  [`{false}`, `{true}`],
  [@bool],

  [Integer],
  [`{10}`, `{0xff}`],
  [@int],

  [Floating-point number],
  [`{3.14}`, `{1e5}`],
  [@float],

  [Length],
  [`{2pt}`, `{3mm}`, `{1em}`, ..],
  [@length],

  [Angle],
  [`{90deg}`, `{1rad}`],
  [@angle],

  [Fraction],
  [`{2fr}`],
  [@fraction],

  [Ratio],
  [`{50%}`],
  [@ratio],

  [String],
  [`{"hello"}`],
  [@str],

  [Label],
  [`{<intro>}`],
  [@label],

  [Math],
  [`[$x^2$]`],
  [@math[Math]],

  [Raw text],
  [``` [`print(1)`]```],
  [@raw],

  [Variable access],
  [`{x}`],
  [@reference:scripting:blocks[Scripting]],

  [Code block],
  [`{{ let x = 1; x + 2 }}`],
  [@reference:scripting:blocks[Scripting]],

  [Content block],
  [`{[*Hello*]}`],
  [@reference:scripting:blocks[Scripting]],

  [Parenthesized expression],
  [`{(1 + 2)}`],
  [@reference:scripting:blocks[Scripting]],

  [Array],
  [`{(1, 2, 3)}`],
  [@array[Array]],

  [Dictionary],
  [`{(a: "hi", b: 2)}`],
  [@dictionary[Dictionary]],

  [Unary operator],
  [`{-x}`],
  [@reference:scripting:operators[Scripting]],

  [Binary operator],
  [`{x + y}`],
  [@reference:scripting:operators[Scripting]],

  [Assignment],
  [`{x = 1}`],
  [@reference:scripting:operators[Scripting]],

  [Field access],
  [`{x.y}`],
  [@reference:scripting:fields[Scripting]],

  [Method call],
  [`{x.flatten()}`],
  [@reference:scripting:methods[Scripting]],

  [Function call],
  [`{min(x, y)}`],
  [@function[Function]],

  [Argument spreading],
  [`{min(..nums)}`],
  [@arguments[Arguments]],

  [Unnamed function],
  [`{(x, y) => x + y}`],
  [@function:unnamed[Function]],

  [Let binding],
  [`{let x = 1}`],
  [@reference:scripting:bindings[Scripting]],

  [Named function],
  [`{let f(x) = 2 * x}`],
  [@function[Function]],

  [Set rule],
  [`{set text(14pt)}`],
  [@reference:styling:set-rules[Styling]],

  [Set-if rule],
  [`{set text(..) if .. }`],
  [@reference:styling:set-rules[Styling]],

  [Show-set rule],
  [`{show heading: set block(..)}`],
  [@reference:styling:show-rules[Styling]],

  [Show rule with function],
  [`{show raw: it => {..}}`],
  [@reference:styling:show-rules[Styling]],

  [Show-everything rule],
  [`{show: template}`],
  [@reference:styling:show-rules[Styling]],

  [Context expression],
  [`{context text.lang}`],
  [@reference:context[Context]],

  [Conditional],
  [`{if x == 1 {..} else {..}}`],
  [@reference:scripting:conditionals[Scripting]],

  [For loop],
  [`{for x in (1, 2, 3) {..}}`],
  [@reference:scripting:loops[Scripting]],

  [While loop],
  [`{while x < 10 {..}}`],
  [@reference:scripting:loops[Scripting]],

  [Loop control flow],
  [`{break, continue}`],
  [@reference:scripting:loops[Scripting]],

  [Return from function],
  [`{return x}`],
  [@function[Function]],

  [Include module],
  [`{include "bar.typ"}`],
  [@reference:scripting:modules[Scripting]],

  [Import module],
  [`{import "bar.typ"}`],
  [@reference:scripting:modules[Scripting]],

  [Import items from module],
  [`{import "bar.typ": a, b, c}`],
  [@reference:scripting:modules[Scripting]],

  [Comment],
  [`{/* block */}`, `{// line}`],
  [@reference:syntax:comments[Below]],
)

= Comments <comments>
Comments are ignored by Typst and will not be included in the output. This is useful to exclude old versions or to add annotations. To comment out a single line, start it with `//`:

```example
// our data barely supports
// this claim

We show with $p < 0.05$
that the difference is
significant.
```

Comments can also be wrapped between `/*` and `*/`. In this case, the comment can span over multiple lines:

```example
Our study design is as follows:
/* Somebody write this up:
   - 1000 participants.
   - 2x2 data design. */
```

= Escape sequences <escapes>
Escape sequences are used to insert special characters that are hard to type or otherwise have special meaning in Typst. To escape a character, precede it with a backslash. To insert any Unicode codepoint, you can write a hexadecimal escape sequence: `[\u{1f600}]`. The same kind of escape sequences also work in @str[strings].

```example
I got an ice cream for
\$1.50! \u{1f600}
```

= Identifiers <identifiers>
Names of variables, functions, and so on (_identifiers_) can contain letters, numbers, hyphens (`-`), and underscores (`_`). They must start with a letter or an underscore.

More specifically, the identifier syntax in Typst is based on the #link("https://www.unicode.org/reports/tr31/")[Unicode Standard Annex \#31], with two extensions: Allowing `_` as a starting character, and allowing both `_` and `-` as continuing characters.

For multi-word identifiers, the recommended case convention is #link("https://en.wikipedia.org/wiki/Letter_case#Kebab_case")[Kebab case]. In Kebab case, words are written in lowercase and separated by hyphens (as in `top-edge`). This is especially relevant when developing modules and packages for others to use, as it keeps things predictable.

```example
#let kebab-case = [Using hyphen]
#let _schön = "😊"
#let 始料不及 = "😱"
#let π = calc.pi

#kebab-case
#if -π < 0 { _schön } else { 始料不及 }
// -π means -1 * π,
// so it's not a valid identifier
```
