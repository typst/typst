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

Typesetting is controlled by hundreds of parameters, from page margins to font 
sizes to numbering conventions. Managing this mess is a major focus of every 
typesetting system. The first step is to arrange these properties into related 
groups. In typst, these groups are called _element functions_ (EFs), for 
example `text` for font properties (typeface, size, color etc.), `par` for 
paragraphs (line spacing, alignment, indentation etc.), and `figure` 
(placement rules, captioning, and numbering of tables and figures). A complete 
list of the available element functions is **where??**. The term _functions_ 
indicates that these entities do not merely act as passive property 
containers, but can actively process content (provided in the function's 
positional argument `body`) according to the present parameter settings.

Since different parts of the document need different settings for the same 
parameters (e.g. the font size in headings, plain text, and footnotes), an 
element function type usually exists in multiple instances. On the other hand, 
different EF types must interact to typeset a particular piece of content. You 
can imagine this as an _ensemble_ of EMs playing together to create the 
desired output. For example, math rendering requires the `math.equation` EF 
for equation-specific information, but also `text` for the font, `par` for 
line spacing, and `block` to control potential page breaks.

The set of active EFs changes frequently during the processing of your 
document. To provide consistent access to precisely the members of the active 
ensemble at a given point, typst uses the concept of a _context_ (see section 
[Context]($context) for detailed information). By default, context expressions 
refer to the context "here" (i.e. to the moment when processing reaches the 
current document location), but you can specify and access many other 
processing stages as well, e.g. "whenever an equation will be rendered in the 
future" or "upon creation of the table-of-contents". Contexts guarantee that 
parameters are accessed in a coordinated way and from the appropriate EF 
instances. 

There are three fundamental methods to modify parameters:
- function calls, e.g. `#text(size: 25pt,  [Hello]))` Use 25pt font to typeset 
just the given content, here 'Hello'.
- set rules, e.g. `#set text(size: 25pt)` Instruct the `text` EF to use 25pt 
font until further notice.
- show rules, e.g. `#show math.equation: set text(size: 25pt)` Typeset 
subsequent equations with 25pt font.
  
Show rules are the most powerful (and most complicated), because they give you 
access to the interacting ensemble of EFs 
in a precisely specified situation (here: equation rendering due to 
`math.equation`). Consistency in a show rule is guaranteed because a show rule 
always defines a context automatically, without the need to type the `context` 
keyword explicitly.

Parameter modifications like the ones above have generally a _limited 
lifetime_: A modification passed as a function argument expires when the 
function completes, and the previous parameter value is restored at this 
point. Set and show rules are active until the end of the enclosing scope, 
i.e. until the next closing bracket or brace:

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

When several rules refering to the same parameter occur in the same scope, 
each one overrides the previous specification. Modifications applied outside 
of any scope, i.e. at the top level of the document, remain active during the 
entire typesetting process unless they are explicitly overridden. These global 
settings are usually provided by a [style template]($tutorial/making-a-template).

Now, let's delve into the details of set and show rules.

## Set rules

Set rules offer the easiest way to customize the appearance of subsequent 
elements.
Their basic syntax resembles a [function call]($function) to an [element
function]($function/#element-functions) preceded by the `[#set]` keyword (or
`{set}` in script mode)

```typ
#set element-function(parameter-spec)     // in markup mode
#{
  set  element-function(parameter-spec)   // in script mode
}
```

`element-function` specifies the EF you want to modify. The `parameter-spec` 
is a sequence of named
parameters with their new values, as in an ordinary function call. The 
supported parameter 
names are the same as in the constructor of the respective EF, as described in 
the 
function's documentation. In the following example, we use two set rules to 
change the heading's
[numbering style]($heading.numbering) and the text's [font 
family]($text.font).

```example
#set heading(numbering: "I.1")
#set text(font: "New Computer Modern")

= Introduction
With set rules, you can style
your document.
```

Note that you cannot pass positional arguments in a set rule &ndash; a set rule
is not really a function call, it just uses the same syntax for convenience.

A set rule refers to all instances of the given EF type and stays in effect 
until the end of the present scope. In particular, a top level set rule 
stays in effect until the end of the file unless explicitly overridden by 
another set rule. To restrict a set rule's lifetime, you can enclose it in 
a code block, i.e. in brackets or braces. Then, the rule expires at the end
of the block, and the previous behavior is restored. Below, we use a content
block to apply the modified list styling only to the list in brackets:

```example
This list is affected: #[
  #set list(marker: [--])
  - Dash
] // end of block, set rule expires

This one is not:
- Bullet
```

The lifetime restriction to the current scope is especially powerful when set 
rules are used inside show rules (see next section): Since a show rule 
implicitly defines a block, any set rule embedded there is only active within 
the show rule's context and does not influence typesetting in other situations.

On the other hand, the lifetime restriction causes a common pitfall when you 
want to apply a set rule conditionally. `set text(red)` in the following code 
has no effect, because it expires before it can influence anything:

```typ
#let task(body, critical: false) = {
  if critical {
    set text(red) 
  } // end of block, set rule expires
  // original behavior restored
  [- #body]
}
```

To avoid this, you must write the condition in postfix notation via a _set-if_ rule:

```example
#let task(body, critical: false) = {
  set text(red) if critical
  [- #body]
}

#task(critical: true)[Food today?]
#task(critical: false)[Work deadline]
```


## Show rules

With show rules, you can deeply customize the typesetting process for
a given type of element. There are two variants of show rules: You 
can specify the desired modifications by a set rule, which is simple 
but of limited expressivity, or by a function, which is more involved 
but unleashes the full range of customization options:

```typ
// in markup mode
#show selector-pattern: set-rule     // set-rule variant
#show selector-pattern: function     // function variant

#{  // likewise in script mode
  show selector-pattern: set-rule     // set-rule variant   
  show selector-pattern: function     // function variant
}
```

The [selector]-pattern specifies the situation where the desired 
modifications shall apply. 
The most common form of `selector-pattern` is an [element 
function]($function/#element-functions) identifier. This means that the 
right-hand side code (the set rule or function) is executed in the context of 
the selected EF, and all modifications expire after completion of this code 
&ndash; modifications in a show rule cannot influence other typesetting 
situations. In the example below, the selector pattern refers
to `heading`, so headings are printed red, while all other text stays black:

```example
#show heading: set text(red)

= First-level headings are red
== Second-level headings are also red
But plain text stays black.
```

You can refine the `selector-pattern` by means of the [where]($function.where) 
function. The arguments of `where` are named parameters with values, and the 
supported parameter names are the same as in the element function's constructor.
The selector pattern then restricts the show rule's scope to the EF instances 
conforming to the given parameter settings. The following example changes
the color only for the first-level headings and leaves everything else unchanged:

```example
#show heading.where(level: 1): set text(red)

= First-level headings are red
== Second-level headings remain black
Plain text stays black as well.
```

A complete list of supported selector patterns is provided below.

To overcome the limitations of set rules on the right-hand side, you use the 
`function` variant of the show rule. In this variant, the right-hand side is 
the name of an arbitrary [function] that accepts exactly one positional 
argument (it can have additional named arguments) and returns arbitrary 
content:

```example
#let always-say-thank-you(it) = {
   it
   set text(green)
   [thank you]
}

#show heading: always-say-thank-you
= This heading is boring
```

The function's argument (conventionally called `it`) is the EF that matched 
the left-hand side of the show rule. The function implements the desired 
modifications (via embedded set and show rules or any other code) and then 
forwards the content for further processing, returns entirely new content, or 
a combination thereof. 

To support advanced customization, you can query the current values of the 
parameter fields of the function's argument (`it.depth` in the example below). 
Likewise, you can query the parameter values of other element functions in the 
show rule's scope, for example the current `text.size`. This is possible 
because a show rule implicitly defines a [Context]($context) to expose this 
information to the user &ndash; outside of a context, these fields are not 
accessible.

```example
#let always-say-thank-you(it) = {
   it
   if it.depth == 1 {
     set text(red,
              size: text.size * 150%)
     [I don't care]
   } else {
     set text(green)
     [thank you]
   }
}

#set heading(numbering: "1.1 ")
#show heading: always-say-thank-you
= This heading is boring
== This one is better
```

In practice, the function is usually implemented in-place as an unnamed function 
(aka. "lambda expression") with the syntax `it => { implementation }`  for an 
implementation in script mode or `it => [ implementation ]` for an 
implementation in markup mode. In this more involved example, we define a 
show rule that formats headings for a fantasy encyclopedia:

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
