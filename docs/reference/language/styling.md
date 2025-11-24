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
`{show}` keyword followed by a [selector], a colon and then a set rule. The most
basic form of selector is an [element function]($function/#element-functions).
This lets the set rule only apply to the selected element. In the example below,
headings become dark blue while all other text stays black.

```example
#show heading: set text(navy)

= This is navy-blue
But this stays black.
```

Note that the [`page`]($page) cannot be targeted by a show rule. Writing `{#show page: ..}` currently has no effect.

With show-set rules you can mix and match properties from different functions to
achieve many different effects. But they still limit you to what is predefined
in Typst. For maximum flexibility, you can instead write a _transformational_
show rule that defines how to format an element from scratch. To write such a
show rule, replace the set rule after the colon with an arbitrary [function].
This function receives the element in question and can return arbitrary content.
The function is often defined inline as `{it => ..}` using the
[unnamed function syntax]($function/#unnamed). The function's parameter is
typically named `it` by convention.

The available [fields]($scripting/#fields) on the element passed to the function
match the parameters of the respective element function. Below, we define a show
rule that formats headings for a fantasy encyclopedia.

The show rule itself adds tilde characters around the title (these must be
escaped with a backslash because otherwise they would indicate a non-breaking
space), emphasizes the title with italics, and then displays the heading counter
after the title.

For this example, we also wanted center alignment and a different font. While we
could've added these set rules into the existing show rule, we instead added
them as separate show-set rules. This is good practice because now these rules
can still be overridden by later show-set rules in the document, keeping styling
composable. In contrast, set rules within a transformational show rule would not
be overridable anymore.

```example
#set heading(numbering: "(I)")
#show heading: set align(center)
#show heading: set text(font: "Inria Serif")
#show heading: it => block[
  \~
  #emph(it.body)
  #counter(heading).display(it.numbering)
  \~
]

= Dragon
With a base health of 15, the dragon is the most
powerful creature.

= Manticore
While less powerful than the dragon, the manticore
gets extra style points.
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

- **Text:** `{show "Text": ..}` \
  Style, transform or replace text.

- **Regex:** `{show regex("\w+"): ..}` \
  Select and transform text with a regular expression for even more flexibility.
  See the documentation of the [`regex` type]($regex) for details.

- **Function with fields:** `{show heading.where(level: 1): ..}` \
  Transform only elements that have the specified fields. For example, you might
  want to only change the style of level-1 headings.

- **Label:** `{show <intro>: ..}` \
  Select and transform elements that have the specified label. See the
  documentation of the [`label` type]($label) for more details.

```example
#show "Project": smallcaps
#show "badly": "great"

We started Project in 2019
and are still working on it.
Project is progressing badly.
```
