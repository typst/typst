<h1 align="center">
  <img alt="Typst" src="https://user-images.githubusercontent.com/17899797/226108480-722b770e-6313-40d7-84f2-26bebb55a281.png">
</h1>

<p align="center">
  <a href="https://typst.app/docs/">
    <img alt="Documentation" src="https://img.shields.io/website?down_message=offline&label=docs&up_color=007aff&up_message=online&url=https%3A%2F%2Ftypst.app%2Fdocs"/>
  </a>
  <a href="https://typst.app/">
    <img alt="Typst App" src="https://img.shields.io/website?down_message=offline&label=typst.app&up_color=239dad&up_message=online&url=https%3A%2F%2Ftypst.app"/>
  </a>
  <a href="https://discord.gg/2uDybryKPe">
    <img alt="Discord Server" src="https://img.shields.io/discord/1054443721975922748?color=5865F2&label=discord&labelColor=555"/>
  </a>
  <a href="https://github.com/typst/typst/blob/main/LICENSE">
    <img alt="Apache-2 License" src="https://img.shields.io/badge/license-Apache%202-brightgreen"/>
  </a>
</p>

Typst is a new markup-based typesetting system that is designed to be as powerful
as LaTeX while being much easier to learn and use. Typst has:

- Built-in markup for the most common formatting tasks
- Flexible functions for everything else
- A tightly integrated scripting system
- Math typesetting, bibliography management, and more
- Fast compile times thanks to incremental compilation
- Friendly error messages in case something goes wrong

This repository contains the Typst compiler and its CLI, which is everything you
need to compile Typst documents locally. For the best writing experience,
consider signing up to our [collaborative online editor][app] for free. It is
currently in public beta.

## Example
A [gentle introduction][tutorial] to Typst is available in our documentation.
However, if you want to see the power of Typst encapsulated in one image, here
it is:
<p align="center">
 <img alt="Example" width="900" src="https://user-images.githubusercontent.com/17899797/228031796-ced0e452-fcee-4ae9-92da-b9287764ff25.png"/>
</p>


Let's dissect what's going on:

- We use _set rules_ to configure element properties like the size of pages or
  the numbering of headings. By setting the page height to `auto`, it scales to
  fit the content. Set rules accommodate the most common configurations. If you
  need full control, you can also use [show rules][show] to completely redefine
  the appearance of an element.

- We insert a heading with the `= Heading` syntax. One equals sign creates a top
  level heading, two create a subheading and so on. Typst has more lightweight
  markup like this, see the [syntax] reference for a full list.

- [Mathematical equations][math] are enclosed in dollar signs. By adding extra
  spaces around the contents of a equation, we can put it into a separate block.
  Multi-letter identifiers are interpreted as Typst definitions and functions
  unless put into quotes. This way, we don't need backslashes for things like
  `floor` and `sqrt`. And `phi.alt` applies the `alt` modifier to the `phi` to
  select a particular symbol variant.

- Now, we get to some [scripting]. To input code into a Typst document, we can
  write a hashtag followed by an expression. We define two variables and a
  recursive function to compute the n-th fibonacci number. Then, we display the
  results in a center-aligned table. The table function takes its cells
  row-by-row. Therefore, we first pass the formulas `$F_1$` to `$F_10$` and then
  the computed fibonacci numbers. We apply the spreading operator (`..`) to both
  because they are arrays and we want to pass the arrays' items as individual
  arguments.

<details>
  <summary>Text version of the code example.</summary>

  ```text
  #set page(width: 10cm, height: auto)
  #set heading(numbering: "1.")

  = Fibonacci sequence
  The Fibonacci sequence is defined through the
  recurrence relation $F_n = F_(n-1) + F_(n-2)$.
  It can also be expressed in _closed form:_

  $ F_n = round(1 / sqrt(5) phi.alt^n), quad
    phi.alt = (1 + sqrt(5)) / 2 $

  #let count = 8
  #let nums = range(1, count + 1)
  #let fib(n) = (
    if n <= 2 { 1 }
    else { fib(n - 1) + fib(n - 2) }
  )

  The first #count numbers of the sequence are:

  #align(center, table(
    columns: count,
    ..nums.map(n => $F_#n$),
    ..nums.map(n => str(fib(n))),
  ))
  ```
</details>

## Install and use
Typst is available through several package managers:

```sh
# macOS or Linux using Homebrew
brew install typst

# Arch Linux
pacman -S typst
```

You can also get sources and pre-built binaries for the latest release of Typst from
the [releases page][releases]. This will give you Typst's CLI which converts
Typst sources into PDFs.

```sh
# Creates `file.pdf` in working directory.
typst file.typ

# Creates PDF file at the desired path.
typst path/to/source.typ path/to/output.pdf
```

You can also watch source files and automatically recompile on changes. This is
faster than compiling from scratch each time because Typst has incremental
compilation.
```sh
# Watches source files and recompiles on changes.
typst --watch file.typ
```

If you prefer an integrated IDE-like experience with autocompletion and instant
preview, you can also check out the [Typst web app][app], which is currently in
public beta.

## Build from source
To build Typst yourself, you need to have the [latest stable Rust][rust]
installed. Then, you can build the CLI with the following command:

```sh
cargo build -p typst-cli --release
```

The optimized binary will be stored in `target/release/`.

## Contributing
We would love to see contributions from the community. If you experience bugs,
feel free to open an issue or send a PR with a fix. For new features, we would
invite you to open an issue first so we can explore the design space together.
If you want to contribute and are wondering how everything works, also check out
the [`ARCHITECTURE.md`][architecture] file. It explains how the compiler works.

## Pronunciation
**IPA**: /taɪpst/

"Ty" like in **Ty**pesetting and "pst" like in Hi**pst**er.

## Design Principles
All of Typst has been designed with three key goals in mind: Power,
simplicity, and performance. We think it's time for a system that matches the
power of LaTeX, is easy to learn and use, all while being fast enough to realize
instant preview. To achieve these goals, we follow three core design principles:

- **Simplicity through Consistency:**
  If you know how to do one thing in Typst, you should be able to transfer that
  knowledge to other things. If there are multiple ways to do the same thing,
  one of them should be at a different level of abstraction than the other. E.g.
  it's okay that `= Introduction` and `#heading[Introduction]` do the same thing
  because the former is just syntax sugar for the latter.

- **Power through Composability:**
  There are two ways to make something flexible: Have a knob for everything or
  have a few knobs that you can combine in many ways. Typst is designed with the
  second way in mind. We provide systems that you can compose in ways we've
  never even thought of. TeX is also in the second category, but it's a bit
  low-level and therefore people use LaTeX instead. But there, we don't really
  have that much composability. Instead, there's a package for everything
  (`\usepackage{knob}`).

- **Performance through Incrementality:**
  All Typst language features must accommodate for incremental compilation.
  Luckily we have [`comemo`], a system for incremental compilation which does
  most of the hard work in the background.

[docs]: https://typst.app/docs/
[app]: https://typst.app/
[discord]: https://discord.gg/2uDybryKPe
[tutorial]: https://typst.app/docs/tutorial/
[show]: https://typst.app/docs/reference/styling/#show-rules
[math]: https://typst.app/docs/reference/math/
[syntax]: https://typst.app/docs/reference/syntax/
[scripting]: https://typst.app/docs/reference/scripting/
[rust]: https://rustup.rs/
[releases]: https://github.com/typst/typst/releases/
[architecture]: https://github.com/typst/typst/blob/main/ARCHITECTURE.md
[`comemo`]: https://github.com/typst/comemo/
