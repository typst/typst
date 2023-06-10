---
description: |
  Are you a LaTeX user? This guide explains the differences and
  similarities between Typst and LaTeX so you can get started quickly.
---

# Guide for LaTeX users
This page is a good starting point if you have used LaTeX before and want to try
out Typst. We will explore the main differences between these two systems from a
user perspective. Although Typst is not built upon LaTeX and has a different
syntax, you will learn how to use your LaTeX skills to get a head start.

<!-- Mention that Typst is not built upon LaTeX -->

Just like LaTeX, Typst is a markup-based typesetting system: You compose your
document in a text file and mark it up with commands and other syntax. Then, you
use a compiler to typeset the source file into a PDF. However, Typst also
differs from LaTeX in several aspects: For one, Typst uses more dedicated syntax
(like you may know from Markdown) for common tasks. Typst's commands are also
more principled: They all work the same, so unlike in LaTeX, you just need to
understand a few general concepts instead of learning different conventions for
each package. Moreover Typst compiles faster than LaTeX: Compilation usually
takes milliseconds, not seconds, so the web app and the compiler can both
provide instant previews.

In the following, we will cover some of the most common questions a user
switching from LaTeX will have when composing a document in Typst. If you prefer
a step-by-step introduction to Typst, check out our [tutorial]($tutorial).

## How do I create a new, empty document? { #getting-started }
That's easy. You just create a new, empty text file (the file extension is
`.typ`). No boilerplate is needed to get started. Simply start by writing your
text. It will be set on an empty A4-sized page. If you are using the web app,
click "+ Empty document" to create a new project with a file and enter the
editor. [Paragraph breaks]($func/parbreak) work just as they do in LaTeX, just
use a blank line.

```example
Hey there!

Here are two paragraphs. The
output is shown to the right.
```

## How do I create section headings, emphasis, ...? { #elements }
LaTeX uses the command `\section` to create a section heading. Nested headings
are indicated with `\subsection`, `\subsubsection`, etc. Depending on your
document class, there is also `\part` or `\chapter`.

In Typst, [headings]($func/heading) are less verbose: You prefix the line with
the heading on it with an equals sign and a space to get a first-order heading:
`[= Introduction]`. If you need a second-order heading, you use two equals
signs: `[== In this paper]`. You can nest headings as deeply as you'd like by
adding more equals signs.

Emphasis (usually rendered as italic text) is expressed by enclosing text in
`[_underscores_]` and strong emphasis (usually rendered in boldface) by using
`[*stars*]` instead.

Here is a list of common markup commands used in LaTeX and their Typst
equivalents. You can also check out the [full syntax cheat sheet]($syntax).

| Element          | LaTeX                     | Typst                  | See                      |
|:-----------------|:--------------------------|:-----------------------|:-------------------------|
| Strong emphasis  | `\textbf{strong}`         | `[*strong*]`           | [`strong`]($func/strong) |
| Emphasis         | `\emph{emphasis}`         | `[_emphasis_]`         | [`emph`]($func/emph)     |
| Monospace / code | `\texttt{print(1)}`       | ``[`print(1)`]``       | [`raw`]($func/raw)       |
| Link             | `\url{https://typst.app}` | `[https://typst.app/]` | [`link`]($func/link)     |
| Label            | `\label{intro}`           | `[<intro>]`            | [`label`]($func/label)   |
| Reference        | `\ref{intro}`             | `[@intro]`             | [`ref`]($func/ref)       |
| Citation         | `\cite{humphrey97}`       | `[@humphrey97]`        | [`cite`]($func/cite)     |
| Bullet list      | `itemize` environment     | `[- List]`             | [`list`]($func/list)     |
| Numbered list    | `enumerate` environment   | `[+ List]`             | [`enum`]($func/enum)     |
| Term list        | `description` environment | `[/ Term: List]`       | [`terms`]($func/terms)   |
| Figure           | `figure` environment      | `figure` function      | [`figure`]($func/figure) |
| Table            | `table` environment       | `table` function       | [`table`]($func/table)   |
| Equation         | `$x$`, `align` / `equation` environments | `[$x$]`, `[$ x = y $]` | [`equation`]($func/math.equation) |

[Lists]($func/list) do not rely on environments in Typst. Instead, they have
lightweight syntax like headings. To create an unordered list (`itemize`),
prefix each line of an item with a hyphen:

````example
To write this list in Typst...

```latex
\begin{itemize}
  \item Fast
  \item Flexible
  \item Intuitive
\end{itemize}
```

...just type this:

- Fast
- Flexible
- Intuitive

````

Nesting lists works just by using proper indentation. Adding a blank line in
between items results in a more [widely]($func/list.tight) spaced list.

To get a [numbered list]($func/enum) (`enumerate`) instead, use a `+` instead of
the hyphen. For a [term list]($func/terms) (`description`), write
`[/ Term: Description]` instead.

## How do I use a command? { #commands }
LaTeX heavily relies on commands (prefixed by backslashes). It uses these
_macros_ to affect the typesetting process and to insert and manipulate content.
Some commands accept arguments, which are most frequently enclosed in curly
braces: `\cite{rasmus}`.

Typst differentiates between [markup mode and code mode]($scripting/#blocks).
The default is markup mode, where you compose text and apply syntactic
constructs such as `[*stars for bold text*]`. Code mode, on the other hand,
parallels programming languages like Python, providing the option to input
and execute segments of code.

Within Typst's markup, you can switch to code mode for a single command (or
rather, _expression_) using a hashtag (`#`). This is how you call functions to,
for example, split your project into different [files]($scripting/#modules)
or render text based on some [condition]($scripting/#conditionals).
Within code mode, it is possible to include normal markup
[_content_]($type/content) by using square brackets. Within code mode, this
content is treated just as any other normal value for a variable.

```example
First, a rectangle:
#rect()

Let me show how to do
#underline([_underlined_ text])

We can also do some maths:
#calc.max(3, 2 * 4)

And finally a little loop:
#for x in range(3) [
  Hi #x.
]
```

A function call always involves the name of the function ([`rect`]($func/rect),
[`underline`]($func/underline), [`calc.max`]($func/calc.max),
[`range`]($func/range)) followed by parentheses (as opposed to LaTeX where the
square brackets and curly braces are optional if the macro requires no
arguments). The expected list of arguments passed within those parentheses
depends on the concrete function and is specified in the
[reference]($reference).

### Arguments { #arguments }
A function can have multiple arguments. Some arguments are positional, i.e., you
just provide the value: The function `[#lower("SCREAM")]` returns its argument
in all-lowercase. Many functions use named arguments instead of positional
arguments to increase legibility. For example, the dimensions and stroke of a
rectangle are defined with named arguments:

```example
#rect(
  width: 2cm,
  height: 1cm,
  stroke: red,
)
```

You specify a named argument by first entering its name (above, it's `width`,
`height`, and `stroke`), then a colon, followed by the value (`2cm`, `1cm`,
`red`). You can find the available named arguments in the [reference
page]($reference) for each function or in the autocomplete panel when typing.
Named arguments are similar to how some LaTeX environments are configured, for
example, you would type `\begin{enumerate}[label={\alph*)}]` to start a list
with the labels `a)`, `b)`, and so on.

Often, you want to provide some [content]($type/content) to a function. For
example, the LaTeX command `\underline{Alternative A}` would translate to
`[#underline([Alternative A])]` in Typst. The square brackets indicate that a
value is [content]($type/content). Within these brackets, you can use normal
markup. However, that's a lot of parentheses for a pretty simple construct.
This is why you can also move trailing content arguments after the parentheses
(and omit the parentheses if they would end up empty).

```example
Typst is an #underline[alternative]
to LaTeX.

#rect(fill: aqua)[Get started here!]
```

### Data types { #data-types }
You likely already noticed that the arguments have distinctive data types. Typst
supports [many data types]($type). Below, there is a table with some of the most
important ones and how to write them. In order to specify values of any of these
types, you have to be in code mode!

| Data type                            | Example                           |
|:-------------------------------------|:----------------------------------|
| [Content]($type/content)             | `{[*fast* typesetting]}`          |
| [String]($type/string)               | `{"Pietro S. Author"}`            |
| [Integer]($type/integer)             | `{23}`                            |
| [Floating point number]($type/float) | `{1.459}`                         |
| [Absolute length]($type/length)      | `{12pt}`, `{5in}`, `{0.3cm}`, ... |
| [Relative length]($type/ratio)       | `{65%}`                           |

The difference between content and string is that content can contain markup,
including function calls, while a string really is just a plain sequence of
characters.

Typst provides [control flow constructs]($scripting/#conditionals) and
[operators]($scripting/#operators) such as `+` for adding things or `==` for
checking equality between two variables. You can also define your own
[variables]($scripting/#bindings) and perform computations on them.

### Commands to affect the remaining document { #rules }
In LaTeX, some commands like `\textbf{bold text}` receive an argument in curly
braces and only affect that argument. Other commands such as
`\bfseries bold text` act as switches, altering the appearance of all subsequent
content within the document or current scope.

In Typst, the same function can be used both to affect the appearance for the
remainder of the document, a block (or scope), or just its arguments. For example,
`[#text(weight: "bold")[bold text]]` will only embolden its argument, while
`[#set text(weight: "bold")]` will embolden any text until the end of the
current block, or, if there is none, document. The effects of a function are
immediately obvious based on whether it is used in a call or a
[set rule.]($styling/#set-rules)

```example
I am starting out with small text.

#set text(14pt)

This is a bit #text(18pt)[larger,]
don't you think?
```

Set rules may appear anywhere in the document. They can be thought of as
default argument values of their respective function:

```example
#set enum(numbering: "I.")

Good results can only be obtained by
+ following best practices
+ being aware of current results
  of other researchers
+ checking the data for biases
```

The `+` is syntactic sugar (think of it as an abbreviation) for a call to the
[`{enum}`]($func/enum) function, to which we apply a set rule above. [Most
syntax is linked to a function in this way.]($syntax) If you need to style an
element beyond what its arguments enable, you can completely redefine its
appearance with a [show rule]($styling/#show-rules) (somewhat comparable to
`\renewcommand`).

## How do I load a document class? { #templates }
In LaTeX, you start your main `.tex` file with the `\documentclass{article}`
command to define how your document is supposed to look. In that command, you
may have replaced `article` with another value such as `report` and `amsart` to
select a different look.

When using Typst, you style your documents with [functions]($type/function).
Typically, you use a template that provides a function that styles your whole
document. First, you import the function from a template file. Then, you apply
it to your whole document. This is accomplished with a
[show rule]($styling/#show-rules) that wraps the following document in a given
function. The following example illustrates how it works:

```example:single
>>> #let conf(
>>>   title: none,
>>>   authors: (),
>>>   abstract: [],
>>>   doc,
>>> ) = {
>>>  set text(font: "Linux Libertine", 11pt)
>>>  set par(justify: true)
>>>  set page(
>>>    "us-letter",
>>>    margin: auto,
>>>    header: align(
>>>      right + horizon,
>>>      title
>>>    ),
>>>    numbering: "1",
>>>  )
>>>
>>>  show heading.where(
>>>    level: 1
>>>  ): it => block(
>>>    align(center,
>>>      text(
>>>        13pt,
>>>        weight: "regular",
>>>        smallcaps(it.body),
>>>      )
>>>    ),
>>>  )
>>>  show heading.where(
>>>    level: 2
>>>  ): it => box(
>>>    text(
>>>      11pt,
>>>      weight: "regular",
>>>      style: "italic",
>>>      it.body + [.],
>>>    )
>>>  )
>>>
>>>  set align(center)
>>>  text(17pt, title)
>>>
>>>  let count = calc.min(authors.len(), 3)
>>>  grid(
>>>    columns: (1fr,) * count,
>>>    row-gutter: 24pt,
>>>    ..authors.map(author => [
>>>      #author.name \
>>>      #author.affiliation \
>>>      #link("mailto:" + author.email)
>>>    ]),
>>>  )
>>>
>>>  par(justify: false)[
>>>    *Abstract* \
>>>    #abstract
>>>  ]
>>>
>>>  set align(left)
>>>  columns(2, doc)
>>>}
<<< #import "conf.typ": conf
#show: conf.with(
  title: [
    Towards Improved Modelling
  ],
  authors: (
    (
      name: "Theresa Tungsten",
      affiliation: "Artos Institute",
      email: "tung@artos.edu",
    ),
    (
      name: "Eugene Deklan",
      affiliation: "Honduras State",
      email: "e.deklan@hstate.hn",
    ),
  ),
  abstract: lorem(80),
)

Let's get started writing this
article by putting insightful
paragraphs right here!
```

The [`{import}`]($scripting/#modules) statement makes
[functions]($type/function) (and other definitions) from another file available.
In this example, it imports the `conf` function from the `conf.typ` file. This
function formats a document as a conference article. We use a show rule to apply
it to the document and also configure some metadata of the article. After
applying the show rule, we can start writing our article right away!

<div class="info-box">

Functions are Typst's "commands" and can transform their arguments to an output
value, including document _content._ Functions are "pure", which means that they
cannot have any effects beyond creating an output value / output content. This
is in stark contrast to LaTeX macros that can have arbitrary effects on your
document.

To let a function style your whole document, the show rule processes everything
that comes after it and calls the function specified after the colon with the
result as an argument. The `.with` part is a _method_ that takes the `conf`
function and pre-configures some if its arguments before passing it on to the
show rule.
</div>

In the web app, you can choose from predefined templates or even
create your own using the template wizard. You can also check out the
[`awesome-typst` repository](https://github.com/qjcg/awesome-typst) to find
templates made by the community. We plan to build a package manager to make
templates even easier to share in the future!

You can also [create your own, custom templates.]($tutorial/making-a-template)
They are shorter and more readable than the corresponding LaTeX `.sty` files by
orders of magnitude, so give it a try!

## How do I load packages? { #packages }
Typst is "batteries included," so the equivalent of many popular LaTeX packages
is built right-in. Below, we compiled a table with frequently loaded packages
and their corresponding Typst functions.

| LaTeX Package                   | Typst Alternative                                                    |
|:--------------------------------|:---------------------------------------------------------------------|
| graphicx, svg                   | [`image`]($func/image) function                                      |
| tabularx                        | [`table`]($func/table), [`grid`]($func/grid) functions               |
| fontenc, inputenc, unicode-math | Just start writing!                                                  |
| babel, polyglossia              | [`text`]($func/text.lang) function: `[#set text(lang: "zh")]`        |
| amsmath                         | [Math mode]($category/math)                                          |
| amsfonts, amssymb               | [`sym`]($category/symbols) module and [syntax]($syntax/#math)        |
| geometry, fancyhdr              | [`page`]($func/page) function                                        |
| xcolor                          | [`text`]($func/text.fill) function: `[#set text(fill: rgb("#0178A4"))]` |
| hyperref                        | [`link`]($func/link) function                                        |
| bibtex, biblatex, natbib        | [`cite`]($func/cite), [`bibliography`]($func/bibliography) functions |
| lstlisting, minted              | [`raw`]($func/raw) function and syntax                               |
| parskip                         | [`block`]($func/block.spacing) and [`par`]($func/par.first-line-indent) functions |
| csquotes                        | Set the [`text`]($func/text.lang) language and type `["]` or `[']`   |
| caption                         | [`figure`]($func/figure) function                                    |
| enumitem                        | [`list`]($func/list), [`enum`]($func/enum), [`terms`]($func/terms) functions |

If you need to load functions and variables from another file, for example, to
use a template, you can use an [`import`]($scripting/#modules) statement. If you
want to include the textual content of another file instead, you can use an
[`{include}`]($scripting/#modules) statement. It will retrieve the content of
the specified file and put it in your document.

Currently, there is no package manager for Typst, but we plan to build one so
that you can easily use packages with tools and templates from the community and
publish your own. Until then, you might want to check out the
[awesome-typst repository](https://github.com/qjcg/awesome-typst), which
compiles a curated list of libraries created for Typst.

## How do I input maths? { #maths }
To enter math mode in Typst, just enclose your equation in dollar signs. You can
enter display mode by adding spaces or newlines between the equation's contents
and its enclosing dollar signs.

```example
The sum of the numbers from
$1$ to $n$ is:

$ sum_(k=1)^n k = (n(n+1))/2 $
```

[Math mode]($category/math) works differently than regular markup or code mode.
Numbers and single characters are displayed verbatim, while multiple consecutive
(non-number) characters will be interpreted as Typst variables.

Typst pre-defines a lot of useful variables in math mode. All Greek (`alpha`,
`beta`, ...) and some Hebrew letters (`alef`, `bet`, ...) are available through
their name. Some symbols are additionally available through shorthands, such as
`<=`, `>=`, and `->`.

Refer to the [symbol page]($func/symbol) for a full list of the symbols.
If a symbol is missing, you can also access it through a
[Unicode escape sequence]($syntax/#escapes).

Alternate and related forms of symbols can often be selected by
[appending a modifier]($type/symbol) after a period. For example,
`arrow.l.squiggly` inserts a squiggly left-pointing arrow. If you want to insert
multiletter text in your expression instead, enclose it in double quotes:

```example
$ delta "if" x <= 5 $
```

In Typst, delimiters will scale automatically for their expressions, just as if
`\left` and `\right` commands were implicitly inserted in LaTeX. You can
customize delimiter behavior using the [`lr` function]($func/math.lr). To
prevent a pair of delimiters from scaling, you can escape them with backslashes.

Typst will automatically set terms around a slash `/` as a fraction while
honoring operator precedence. All round parentheses not made redundant by the
fraction will appear in the output.

```example
$ f(x) = (x + 1) / x $
```

[Sub- and superscripts]($func/math.attach) work similarly in Typst and LaTeX.
`{$x^2$}` will produce a superscript, `{$x_2$}` yields a subscript. If you want
to include more than one value in a sub- or superscript, enclose their contents
in parentheses: `{$x_(a -> epsilon)$}`.

Since variables in math mode do not need to be preprended with a `#` or a `/`,
you can also call functions without these special characters:

```example
$ f(x, y) := cases(
  1 "if" (x dot y)/2 <= 0,
  2 "if" x "is even",
  3 "if" x in NN,
  4 "else",
) $
```

The above example uses the [`cases` function]($func/math.cases) to describe f. Within
the cases function, arguments are delimited using commas and the arguments are
also interpreted as math. If you need to interpret arguments as Typst
values instead, prefix them with a `#`:

```example
$ (a + b)^2
  = a^2
  + text(fill: #maroon, 2 a b)
  + b^2 $
```

You can use all Typst functions within math mode and insert any content. If you
want them to work normally, with code mode in the argument list, you can prefix
their call with a `#`. Nobody can stop you from using rectangles or emoji as
your variables anymore:

```example
$ sum^10_(🥸=1)
  #rect(width: 4mm, height: 2mm)/🥸
  = 🧠 maltese $
```

If you'd like to enter your mathematical symbols directly as Unicode, that is
possible, too!

Math calls can have two-dimensional argument lists using `;` as a delimiter. The
most common use for this is the [`mat` function]($func/math.mat) that creates
matrices:

```example
$ mat(
  1, 2, ..., 10;
  2, 2, ..., 10;
  dots.v, dots.v, dots.down, dots.v;
  10, 10, ..., 10;
) $
```

## How do I get the "LaTeX look?" { #latex-look }
Papers set in LaTeX have an unmistakeable look. This is mostly due to their
font, Computer Modern, justification, narrow line spacing, and wide margins.

The example below
- sets wide [margins]($func/page.margin)
- enables [justification]($func/par.justify), [tighter lines]($func/par.leading)
  and [first-line-indent]($func/par.first-line-indent)
- [sets the font]($func/text.font) to "New Computer Modern", an OpenType
  derivate of Computer Modern for both text and [code blocks]($func/raw)
- disables paragraph [spacing]($func/block.spacing)
- increases [spacing]($func/block.spacing) around [headings]($func/heading)

```typ
#set page(margin: 1.75in)
#set par(leading: 0.55em, first-line-indent: 1.8em, justify: true)
#set text(font: "New Computer Modern")
#show raw: set text(font: "New Computer Modern Mono")
#show par: set block(spacing: 0.55em)
#show heading: set block(above: 1.4em, below: 1em)
```

This should be a good starting point! If you want to go further, why not create
a reusable template?

## What limitations does Typst currently have compared to LaTeX? { #limitations }
Although Typst can be a LaTeX replacement for many today, there are still
features that Typst does not (yet) support. Here is a list of them which, where
applicable, contains possible workarounds.

- **Native charts and plots.** LaTeX users often create charts along with their
  documents in PGF/TikZ. Typst does not yet include tools to draw diagrams, but
  the community is stepping up with solutions such as
  [`typst-canvas`](https://github.com/johannes-wolf/typst-canvas),
  [`typst-plot`](https://github.com/johannes-wolf/typst-plot), and
  [`circuitypst`](https://github.com/fenjalien/circuitypst). You can add those
  to your document to get started with drawing diagrams.

- **Change page margins without a pagebreak.** In LaTeX, margins can always be
  adjusted, even without a pagebreak. To change margins in Typst, you use the
  [`page` function]($func/page) which will force a page break. If you just want
  a few paragraphs to stretch into the margins, then reverting to the old
  margins, you can use the [`pad` function]($func/pad) with negative padding.

- **Floating figures.** The figure command of LaTeX will smartly choose where to
  place a figure, be it on the top or bottom of the page, or a dedicated figure
  page. Typst's figure will always appear at the spot where they have been
  inserted in the markup. While this behavior can save some headache, it is
  often cumbersome to manually place figures. We will be adding this feature
  soon!

- **Include PDFs as images.** In LaTeX, it has become customary to insert vector
  graphics as PDF or EPS files. Typst supports neither format as an image
  format, but you can easily convert both into SVG files with [online
  tools](https://cloudconvert.com/pdf-to-svg) or
  [Inkscape](https://inkscape.org/). We plan to add automatic conversion for
  these file formats to the Typst web app, too!

- **Page break optimization.** LaTeX runs some smart algorithms to not only
  optimize line but also page breaks. While Typst tries to avoid widows and
  orphans, it uses less sophisticated algorithms to determine page breaks. You
  can insert custom page breaks in Typst using `[#pagebreak(weak: true)]` before
  submitting your document. The argument `weak` ensures that no double page
  break will be created if this spot would be a natural page break
  anyways. You can also use `[#v(1fr)]` to distribute space on your page. It
  works quite similar to LaTeX's `\vfill`.

- **Bibliographies are not customizable.** In LaTeX, the packages `bibtex`,
  `biblatex`, and `natbib` provide a wide range of reference and bibliography
  formats. You can also use custom `.bbx` files to define your own styles there.
  Typst only supports a small set of citation styles at the moment, but we want
  to build upon this by supporting [Citation Style Language
  (CSL)](https://citationstyles.org), an XML-based format backed by Zotero that
  allows you to describe your own bibliography styles.
