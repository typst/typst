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

## The computational model behind typst

Typesetting is controlled by hundreds of parameters, from page margins to font sizes to numbering conventions. Managing this mess is a major focus of every typesetting system. The first step is to arrange these properties into related groups. In typst, these groups are called _element functions_ (EFs), for example `text` for font properties (typeface, size, color etc.), `par` for paragraphs (line spacing, alignment, indentation etc.), and `figure` (placement rules, captioning, and numbering of tables and figures). A complete list of the available element functions is **where??**. The term _functions_ indicates that these entities do not merely act as passive property containers, but can actively process content (usually provided in the function's `body` parameter) according to the present parameter settings.

Since different parts of the document need different settings for the same parameters (e.g. the font size in headings, plain text, and footnotes), each element function type exists in multiple instances. On the other hand, different EF types must interact to typeset a particular piece of content. You can imagine this as an _ensemble_ of EMs playing together to create the desired output. For example, math rendering requires the `math.equation` EF for equation-specific information, but also `text` for the font, `par` for line spacing, and `block` to control potential page breaks. A complete table of which EFs interact in which situation can be found **where??**.

The set of active EFs changes frequently during the processing of your document. To provide consistent access to precisely the members of the active ensemble at a given point, typst uses the concept of a _context_ (see section [Context]($context) for detailed information). By default, typst expressions refer to the context "here" (i.e. to the moment when processing reaches the current document location), but you can specify and access many other processing stages as well, e.g. "whenever an equation will be rendered in the future" or "upon creation of the table-of-contents". By accessing element functions via contexts, it is guaranteed that parameters are manipulated in a coordinated way and in the appriate instances of the involved EFs. 

There are three fundamental methods to modify parameters:
- function calls, e.g. `#text(size: 25pt,  [Hello]))` Use 25pt font to typeset just the given content, here 'Hello'.
- set rules, e.g. `#set text(size: 25pt)` Instruct the currently active `text` EF to use 25pt font until further notice.
- show rules, e.g. `#show math.equation: text.with(size: 25pt)` Typeset subsequent equations with 25pt font.
  
Show rules are the most powerful (and most complicated), because they give you access to an entire active ensemble of EFs. Parameter modifications like the ones above have generally a _limited lifetime_: A modification passed as a function argument expires when the function completes, and the original parameter value is restored. Set and show rules are active until the end of the enclosing scope, i.e. until the next closing bracket or brace:

```example
#set text(size: 11pt) // #1
this is 11pt \
#[ // open a new scope
   still 11pt \
   #set text(size: 25pt) // #2
   now 25pt \
]  // end of scope, #2 expires
// #1 reactivated
again 11pt
```

When several rules refering to the same parameter occur in the same scope, each one overrides the previous specification. Modifications applied outside of any scope, i.e. at the top level of the document, remain active during the entire typesetting process unless they are explicitly overridden. These global settings are usually provided by a style template.

Scopes give rise to a common pitfall when you want to apply rules conditionally. The following code has no effect:

```typ
#if some-condition() {
   set text(size: 11pt) // #1
} else { // end of scope, #1 expires
   set text(size: 25pt) // #2
} // end of scope, #2 expires
// original behavior restored
```

You must use the "set-if rule" instead

```typ
#set text(size: 11pt) if some-condition()
#set text(size: 25pt) if not some-condition()
// no scope, modification is active
```

Now, let's delve into the details of set and show rules.

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

With show-set rules you can mix and match properties from different functions to
achieve many different effects. But they still limit you to what is predefined
in Typst. For maximum flexibility, you can instead write a show rule that
defines how to format an element from scratch. To write such a show rule,
replace the set rule after the colon with an arbitrary [function]. This function
receives the element in question and can return arbitrary content. The available
[fields]($scripting/#fields) on the element passed to the function again match
the parameters of the respective element function. Below, we define a show rule
that formats headings for a fantasy encyclopedia.

```example
#set heading(numbering: "(I)")
#show heading: it => [
  #set align(center)
  #set text(font: "Inria Serif")
  \~ #emph(it.body)
     #counter(heading).display(
       it.numbering
     ) \~
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
