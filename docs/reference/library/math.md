Typst has special [syntax]($syntax/#math) and library functions to typeset
mathematical formulas. Math formulas can be displayed inline with text or as
separate blocks. They will be typeset into their own block if they start and end
with at least one space (e.g. `[$ x^2 $]`).

# Variables
In math, single letters are always displayed as is. Multiple letters, however,
are interpreted as variables and functions. To display multiple letters
verbatim, you can place them into quotes and to access single letter variables,
you can use the [hash syntax]($scripting/#expressions).

```example
$ A = pi r^2 $
$ "area" = pi dot "radius"^2 $
$ cal(A) :=
    { x in RR | x "is natural" } $
#let x = 5
$ #x < 17 $
```

# Symbols
Math mode makes a wide selection of [symbols]($category/symbols/sym) like `pi`,
`dot`, or `RR` available. Many mathematical symbols are available in different
variants. You can select between different variants by applying
[modifiers]($symbol) to the symbol. Typst further recognizes a number of
shorthand sequences like `=>` that approximate a symbol. When such a shorthand
exists, the symbol's documentation lists it.

```example
$ x < y => x gt.eq.not y $
```

# Line Breaks
Formulas can also contain line breaks. Each line can contain one or multiple
_alignment points_ (`&`) which are then aligned.

```example
$ sum_(k=0)^n k
    &= 1 + ... + n \
    &= (n(n+1)) / 2 $
```

# Function calls
Math mode supports special function calls without the hash prefix. In these
"math calls", the argument list works a little differently than in code:

- Within them, Typst is still in "math mode". Thus, you can write math directly
  into them, but need to use hash syntax to pass code expressions (except for
  strings, which are available in the math syntax).
- They support positional and named arguments, as well as argument spreading.
- They don't support trailing content blocks.
- They provide additional syntax for 2-dimensional argument lists. The semicolon
  (`;`) merges preceding arguments separated by commas into an array argument.

```example
$ frac(a^2, 2) $
$ vec(1, 2, delim: "[") $
$ mat(1, 2; 3, 4) $
$ mat(..#range(1, 5).chunks(2)) $
$ lim_x =
    op("lim", limits: #true)_x $
```

To write a verbatim comma or semicolon in a math call, escape it with a
backslash. The colon on the other hand is only recognized in a special way if
directly preceded by an identifier, so to display it verbatim in those cases,
you can just insert a space before it.

Functions calls preceded by a hash are normal code function calls and not
affected by these rules.

# Alignment
When equations include multiple _alignment points_ (`&`), this creates blocks of
alternatingly right- and left-aligned columns. In the example below, the
expression `(3x + y) / 7` is right-aligned and `= 9` is left-aligned. The word
"given" is also left-aligned because `&&` creates two alignment points in a row,
alternating the alignment twice. `& &` and `&&` behave exactly the same way.
Meanwhile, "multiply by 7" is right-aligned because just one `&` precedes it.
Each alignment point simply alternates between right-aligned/left-aligned.

```example
$ (3x + y) / 7 &= 9 && "given" \
  3x + y &= 63 & "multiply by 7" \
  3x &= 63 - y && "subtract y" \
  x &= 21 - y/3 & "divide by 3" $
```

# Math fonts
The default math font is `New Computer Modern Math`. As demonstrated below, you
can tweak it with [show-set rules]($styling/#show-rules). The rule's selector
can be the general `{math.equation}`, specific [symbols](#symbols) and texts, or
[math functions](#function-calls) like `{math.op}`.

```example:"Change the overall math font"
#show math.equation: set text(font: "Fira Math")
$ sum_(i in NN) 1 + i $
```

```example:"Change the font for a specific character"
#show math.equation: it => {
  show "{": set text(font: "STIX Two Math", fill: maroon)
  it
}
$ f(x, y) := cases(0 "if" x < 0, x "otherwise") $
```

As in the regular text layout, the [`font`]($text.font) parameter also accepts
a priority list of font family descriptor. In the example below, the font
`Noto Sans Math` covers capital serif italic letters, and the default font
`New Computer Modern Math` covers the others. The letters `𝐴` and `𝑍` in the
[regex] are [mathematical alphanumeric symbols](https://en.wikipedia.org/wiki/Mathematical_Alphanumeric_Symbols)
defined in the Unicode standard, instead of the regular `A` and `Z` in ASCII.
Besides, some characters do not belong to this Unicode block for historic
reasons, making it harder to match other ranges. For example, `[𝑎-𝑧]` does not
match `ℎ` and the dotless `𝚤` and `𝚥`.

```example:"Change the font for a range of characters"
#show math.equation: set text(font: (
  (name: "Noto Sans Math", covers: regex("[𝐴-𝑍]")),
  "New Computer Modern Math",
))
$ 2A + B = C. $
```

In addition to [`font`]($text.font), the rule can also change OpenType
[features]($text.features) including [stylistic sets]($text.stylistic-set) and
character variants.

```example:"Configure OpenType features"
#show math.equation: set text(
  // Prefer upright integrals (ss02) and small capitals (ss05)
  stylistic-set: (2, 5),
  // Use a slashed circle for ∅, replacing the default slashed zero
  features: ("cv01",),
)
// These features are defined by New Computer Modern Math.
// Other fonts may not support them or map them to different features.

$ integral f dif x, a inter bb(N) = nothing. $
```

Note that typesetting maths involves complex positioning and spacing, requiring
a specially designed [OpenType math font](https://learn.microsoft.com/typography/opentype/spec/math).
Typst will take the first font without any [`covers`]($text.font) as the base
font, and extract typographic metrics for maths from it. If you have to use
non-math fonts for certain glyphs, specify a coverage to tell Typst select the
base from other fonts.

```example
#show math.equation: set text(font: (
  (name: "New Computer Modern Math", covers: "latin-in-cjk"),
  (name: "Noto Serif CJK SC", covers: regex(".")),
  "New Computer Modern Math",
))
$ a' star b = b' star a. "（“乘法”交换律）" $
```

# Math module
All math functions are part of the `math` [module]($scripting/#modules), which
is available by default in equations. Outside of equations, they can be accessed
with the `math.` prefix.

# Accessibility
To make math accessible, you must provide alternative descriptions of equations
in natural language using the [`alt` parameter of
`math.equation`]($math.equation.alt). For more information, see the [Textual
Representations section of the Accessibility
Guide]($guides/accessibility/#textual-representations).

```example
#math.equation(
  alt: "d S equals delta q divided by T",
  $ d "S" = (delta q) / T $,
)
```

In the future, Typst will automatically make equations without alternative
descriptions accessible in HTML and PDF 2.0 export.
