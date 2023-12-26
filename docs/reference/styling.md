---
description: All concepts needed to style your document with Typst.
---

# Styling
Typst includes a flexible styling system that automatically applies styling of
your choice to your document. With _set rules,_ you can configure basic
properties of elements. This way, you create most common styles. However, there
might not be a built-in property for everything you wish to do. For this reason,
Typst further supports _show rules_ that can completely redefine the appearance
of elements.

## Set rules
With set rules, you can customize the appearance of elements. They are written
as a [function call]($function) to an [element
function]($function/#element-functions) preceded by the `{set}` keyword (or
`[#set]` in markup). Only optional parameters of that function can be provided
to the set rule. Refer to each function's documentation to see which parameters
are optional. In the example below, we use two set rules to change the
[font family]($text.font) and [heading numbering]($heading.numbering).

```example
#set heading(numbering: "I.")
#set text(
  font: "New Computer Modern"
)

= Introduction
With set rules, you can style
your document.
```

A top level set rule stays in effect until the end of the file. When nested
inside of a block, it is only in effect until the end of that block. With a
block, you can thus restrict the effect of a rule to a particular segment of
your document. Below, we use a content block to scope the list styling to one
particular list.

```example
This list is affected: #[
  #set list(marker: [--])
  - Dash
]

This one is not:
- Bullet
```

Sometimes, you'll want to apply a set rule conditionally. For this, you can use
a _set-if_ rule.

```example
#let task(body, critical: false) = {
  set text(red) if critical
  [- #body]
}

#task(critical: true)[Food today?]
#task(critical: false)[Work deadline]
```

## Show rules
With show rules, you can deeply customize the look of a type of element. The
most basic form of show rule is a _show-set rule._ Such a rule is written as the
`{show}` keyword followed by a [selector]($selector), a colon and then a set
rule. The most basic form of selector is an
[element function]($function/#element-functions). This lets the set rule only
apply to the selected element. In the example below, headings become dark blue
while all other text stays black.

```example
#show heading: set text(navy)

= This is navy-blue
But this stays black.
```

With show-set rules you can mix and match properties from different functions to
achieve many different effects. But they still limit you to what is predefined
in Typst. For maximum flexibility, you can instead write a show rule that
defines how to format an element from scratch. To write such a show rule,
replace the set rule after the colon with an arbitrary [function]($function).
This function receives the element in question and can return arbitrary content.
Different [fields]($scripting/#fields) are available on the element passed to
the function. Below, we define a show rule that formats headings for a fantasy
encyclopedia.

```example
#set heading(numbering: "(I)")
#show heading: it => [
  #set align(center)
  #set text(font: "Inria Serif")
  \~ #emph(it.body)
     #counter(heading).display() \~
]

= Dragon
With a base health of 15, the
dragon is the most powerful
creature.

= Manticore
While less powerful than the
dragon, the manticore gets
extra style points.
```

Like set rules, show rules are in effect until the end of the current block or
file.

Instead of a function, the right-hand side of a show rule can also take a
literal string or content block that should be directly substituted for the
element. And apart from a function, the left-hand side of a show rule can also
take a number of other _selectors_ that define what to apply the transformation
to:

- **Everything:** `{show: rest => ..}` \
  Transform everything after the show rule. This is useful to apply a more
  complex layout to your whole document without wrapping everything in a giant
  function call.

  ```example
  #show selector(<center>): it => {
    align(center, it)
  }
  
  #show selector(<first_use>): it => {
    text(style: "italic", it)
  }

  = Counter <center>
  Counter <first_use>
  counts through pages, elements and more.
  ```

- **Text:** `{show "Text": ..}` \
  Style, transform or replace text.

  ```example
  #show "C++": box // To ensure C++ are in the same line

  Rust is designed to be fast, both in terms of runtime performance and in terms of compile times. It is often used for systems programming tasks where performance is critical. The compiled code is about the same performance as C/C++, and is excellent in memory and energy efficiencies.
  ```

- **Regex:** `{show regex("\w+"): ..}` \
  Select and transform text with a regular expression for even more flexibility.
  See the documentation of the [`regex` type]($regex) for details.

  ```example
  #show regex("\d+"): set text(red)
  
  The numbers 1 to 10.  
  ```

- **Function with fields:** `{show heading.where(level: 1): ..}` \
  Transform only elements that have the specified fields. For example, you might
  want to only change the style of level-1 headings.

  ```example
  #show figure.where(
    kind: table
  ): set figure.caption(position: top)
  
  #figure(
    table(
      columns: (auto, auto),
      align: auto,
      
      /* Header*/
      [*\#*], [*Name*],
      
      /* Body */
      [1], [Python],
      [2], [Rust],
    ),
    kind: table,
    caption: "Programming languages"
  )
  ```

- **Label:** `{show <intro>: ..}` \
  Select and transform elements that have the specified label. See the
  documentation of the [`label` type]($label) for more details.

  ```example
  #show <b>: set text(weight: "bold")
  
  Hello,
  World! <b>
  
  Hello, World! <b>  
  ```

```example
#show "Project": smallcaps
#show "badly": "great"

We started Project in 2019
and are still working on it.
Project is progressing badly.
```
