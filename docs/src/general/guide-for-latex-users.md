---
description: |
  Are you a LaTeX user? This guide explains the differences and
  similarities between Typst and LaTeX so you can get started quickly.
---

# Guide for LaTeX users
This page is a good starting point if you have used LaTeX before and want
to try Typst. We will explore the main differences between these two
systems from a user perspective and you will learn how to use your LaTeX
skills to get a head start with Typst.

<!-- Mention that Typst is not built upon LaTeX -->

Just like LaTeX, Typst is a markup-based typesetting system: You compose your
document in a text file and mark it up with commands, macros, or other syntax
and a compiler will typeset it into a PDF file. Typst differs from LaTeX by
using more syntax (like you may know from Markdown) for common tasks. Typst's
compilation times are also usually measured in milliseconds, not seconds, so the
web app or the compiler can provide instant previews.

In the following, we will cover some of the most common tasks and questions that
occur while composing a basic document.

## How do I create a new, empty document?
That's easy. You just create a new, empty text file (the file extension is
`.typ`). No boilerplate is needed to start writing, instead, your text will be
set on a default A4 page. If you are using the web app, you'll just need to
click "Create document".

```example
Hello, world!
```

Paragraph breaks work just as they do in LaTeX, just use a blank line.

## How do I create a section heading, emphasis, ...?
LaTeX uses the command `\section` to create a section heading. If you want to go
deeper, you could use `\subsection`, `\subsubsection`, etc. Depending on your
document class, you could also use `\part` or `\chapter`

In Typst, headings are less verbose: You prefix the line with the heading on it
with an equals sign and a space to get a first-order heading: `[=
Introduction]`. If you need a second-order heading, you use two equals signs:
`[== In this paper]`. You can nest headings as deeply as you'd like by adding
more equals signs.

Emphasis (usually rendered as italic text) is expressed by enclosing text in
`[_underscores_]` and strong emphasis (usually rendered in boldface) by using
`[*stars*]` instead.

Below, there is a comparison between LaTeX commands and their syntactic Typst
equivalents:

| Action           | LaTeX                     | Typst                  |
|:-----------------|:--------------------------|:-----------------------|
| Strong emphasis  | `\textbf{strong}`         | `[*strong*]`           |
| Emphasis         | `\emph{emphasis}`         | `[_emphasis_]`         |
| Monospace / code | `\texttt{print(1)}`       | ``[`print(1)`]``       |
| Link             | `\url{https://typst.app}` | `[https://typst.app/]` |
| Label            | `\label{intro}`           | `[<intro>]`            |
| Reference        | `\ref{intro}`             | `[@intro]`             |
| Citation         | `\cite{humphrey97}`       | `[@humphrey97]`        |

Lists do not rely on environments like `\begin{itemize}\item List
item\end{itemize}`. Instead, they work like headings. You need to prefix the
line of the list item with a hyphen: `[- A bag of milk]`. This will produce an
unordered list (`itemize`). To get a numbered list (`enumerate`) instead, use a
`+` instead of the hyphen. To nest lists, you can just indent the lines to be
nested with tabs or spaces.

Learn more:
- [Headings]($func/heading)
- [Unordered lists]($func/list)
- [Ordered lists]($func/enum)
- [Full syntax cheatsheet]($syntax)

## How do I load a document class?
In LaTeX, you start your main `.tex` file with the `\documentclass{article}`
command to set how your document is supposed to look. In that command, you may
have replaced `article` with another value such as `report` and `amsart` to
customize your document.

In Typst, you instead import a function that styles its argument. Then, you wrap
your document in it with an "everything" show rule, which wraps the following
document in a given function.

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

Let's get started writing this article!
```

The `{import}` statement makes functions from another file available, in this
case the `conf` function that formats the document as a conference article. We
pass some metadata about the article to that function. Finally, we can get
started writing our article below!

You can create a document from a template in the template gallery in the web app
or even create your own using the template wizard. You can also check out the
[awesome-typst repository](https://github.com/qjcg/awesome-typst) to check out
some templates by the community. We plan to build a package manager to make
templates even easier to use in the future!

You can also create your own templates. They often are shorter and more readable
than the corresponding LaTeX `.sty` files by orders of magnitude, so give it a
try!

Learn more:
- [Show rules]($styling/#show-rules)
- [Import statements and modules]($scripting/#modules)
- [Template section of the tutorial]($tutorial/making-a-template)

## How do I use a command?
LaTeX heavily relies on commands, prefixed by backslashes. It uses these
_macros_ to affect the typesetting process, insert, and manipulate content. Some
commands accept arguments, most frequently they are enclosed in curly braces:
`\cite{rasmus}`. Within Typst's markup, function calls and keywords need a
leading hashtag. All function calls need an argument list enclosed in
parentheses, even if it is empty: `#rect()`.

### Arguments
A function can have multiple arguments. Some arguments are positional, i.e. you
just provide the value: The function `[#lower(1)]` determines if its argument
is an odd number and returns `{true}` or `{false}`. Many functions use named
arguments instead of positional arguments if they can receive a lot of
configuration or if some arguments are used less often. For example, the
dimensions and stroke of a rectangle are defined with named arguments:

```example
#rect(width: 2cm, height: 1cm, stroke: red)
```

You specify a named argument by first specifying its name, then a colon,
followed by the value. You can find the available named arguments in the
reference page for each function or in the autocomplete panel when typing. Named
arguments are similar to how some LaTeX environments are configured, for
example, you would type `\begin{enumerate}[label={\alph*)}]` to start a list
with the labels `a)`, `b)`, and so on.

Often, you want to provide some content to a function. For example, the LaTeX
command `\underline{Alternative A}` would translate to `[#underline([Alternative
A])]` in Typst. The square brackets indicate that a value is content. Within these
brackets, you can use normal markup. However, that's a lot of parentheses for a
pretty simple call. This is why you can also omit the parentheses in favor of
the square brackets to pass a _content_ value as the last positional argument.

```example
Typst is an #underline[alternative]
to LaTeX.

#rect(stroke: 2pt)[Get started here!]
```

### Data types
You likely already noticed that the arguments have distinctive data types.
Typst supports many data types, below, there is a table with a few of the
most important ones and how to write them:

| Data type             | Example                           |
|:----------------------|:----------------------------------|
| Content               | `{[*fast* typesetting]}`          |
| String                | `{"Pietro S. Author"}`            |
| Integer               | `{23}`                            |
| Floating point number | `{1.459}`                         |
| Absolute length       | `{12pt}`, `{5in}`, `{0.3cm}`, ... |
| Relative length       | `{65%}`                           |

The difference between content and string is that content can contain markup and
other commands while a string really is just a sequence of characters. You can
use operators like `+` for summation and `==` for equality on these types like
you would in a conventional programming language instead of using
`\addtocounter` or `\ifnum`. You can even assign and use custom variables!

In order to specify values of any of these types, you have to be in a coding
context. You previously saw that they apply in argument lists. In Markup, the
`#` character invokes code mode for a single expression (like a function call,
variable assignment, etc.). If you want to switch to code mode for multiple
expressions, you can use a code block with `[#{ ... }]`. You can now put
multiple expressions in the curly braces.

### Commands to affect the remaining document
In LaTeX, some commands like `\textbf{bold text}` are passed their argument in curly
braces and only affect that argument whereas other commands like `\bfseries bold
text` act as switches and change the appearance of all following content in the
document or the current scope (denoted by a set of curly braces).

In Typst, functions can be used in both ways: With effects applying until the
end of the document or block or just to its arguments. For example,
`[#text(weight: "bold")[bold text]]` will only embolden its argument, while
`[#set text(weight: "bold")]` will embolden any text until the end of the
current block, or, if there is none, document. The effects of a function are
immediately obvious depending on if it is used in a call or a set rule.

```example
I am starting out with small text.

#set text(14pt)

This is a bit #text(18pt)[larger,]
don't you think?
```

Set rules may appear anywhere in the document and can be though of as
pre-setting the arguments of their function:

```example
#set enum(numbering: "I.")

Good results can only be obtained by
+ following best practices
+ being aware of current results of other
  researchers
+ checking the data for biases
```

The `+` is syntactic sugar for a call to the `{enum}` function, to which we
apply a set rule above. Most syntax is linked to a function it desugars to.
If you need to style an element beyond what its arguments enable, you can
completely redefine its appearance with a `show` rule, which is comparable to
`\renewcommand`.

## How do I load packages?
Most packages you load in LaTeX are just included in Typst, no need to load or
install anything. Below, we compiled a table with frequently loaded packages and
their corresponding Typst functions.

| LaTeX Package                   | Typst Alternative                    |
|:--------------------------------|:-------------------------------------|
| graphicx, svg                   | `image` function                     |
| tabularx                        | `table`, `grid` functions            |
| fontenc, inputenc, unicode-math | Just start writing!                  |
| babel, polyglossia              | `[#set text(lang: "zh")]`            |
| amsmath                         | Math mode                            |
| amsfonts, amssymb               | `sym` module and syntax              |
| geometry, fancyhdr              | `page` function                      |
| xcolor                          | `[#set text(fill: rgb("#0178A4"))]`  |
| hyperref                        | `link` function                      |
| bibtex, biblatex, natbib        | `cite`, `bibliography` functions     |
| lstlisting, minted              | `raw` function and syntax            |
| parskip                         | `[#show par: set block(spacing: 16pt)]` and `[#set par(first-line-indet: 12pt)]` |
| csquotes                        | `[#set text(lang: "de")]` and type `["]` or `[']` |
| caption                         | `figure` function                    |
| enumitem                        | `list`, `enum`, `terms` functions    |

If you need to load functions and variables from another file, for example to
use a template, you can use an `{import}` statement:

- `[#import "file.typ"]` imports all functions and variables defined in
  `file.typ` and puts them in a module. For example, if `file.typ` defined the
  function `blockquote`, we could use it as `#file.blockquote[My quote]` after
  the import statement.
- `[#import "file.typ": blockquote, sparkles]` imports `blockquote` and
  `sparkles` from `file.typ`. They are put in the current scope, so you can use
  them directly: `[#blockquote[Always obey your parents when they are present.]]`
- `[#import "file.typ": *]` imports all functions and variables defined in
  `file.typ`. They are put in the current scope, so you can use them directly,
  just as above.

If you want to include the content of another file instead, you can use an
`{include}` expression. It will yield the content of the included file and put
it in your document.

```example
>>> = The Period
>>> It was the best of times, it was the worst of times, it was the age of wisdom, it was the age of foolishness, it was the epoch of belief, it was the epoch of incredulity, it was the season of Light, it was the season of Darkness, it ...
<<< #include "chap1.typ"
```

## How do I input maths?
To enter math mode in Typst, just enclose your equation in dollar signs. You can
enter display mode by putting spaces or newlines between the opening and closing
dollar sign and the equation.

```example
$1+1=2$
$ a x $
```


