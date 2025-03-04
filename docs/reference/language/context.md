---
description: |
   How to deal with content that reacts to its location in the document.
---

# Context
Sometimes, we want to create content that reacts to its location in the
document. This could be a localized phrase that depends on the configured text
language or something as simple as a heading number which prints the right
value based on how many headings came before it. However, Typst code isn't
directly aware of its location in the document. Some code at the beginning of
the source text could yield content that ends up at the back of the document.

To produce content that is reactive to its surroundings, we must thus
specifically instruct Typst: We do this with the `{context}` keyword, which
precedes an expression and ensures that it is computed with knowledge of its
environment. In return, the context expression itself ends up opaque. We cannot
directly access whatever results from it in our code, precisely because it is
contextual: There is no one correct result, there may be multiple results in
different places of the document. For this reason, everything that depends on
the contextual data must happen inside of the context expression.

Aside from explicit context expressions, context is also established implicitly
in some places that are also aware of their location in the document:
[Show rules]($styling/#show-rules) provide context[^1] and numberings in the
outline, for instance, also provide the proper context to resolve counters.

## Behavior of the context keyword
Style properties frequently change within a document, for example by applying set 
rules. To retrieve such poperties in a consistent way, one must first specify 
the context where the query is to be executed. This is the purpose of the 
`context` keyword. Once the context has been fixed, the property information 
is available through a simple field access syntax. For example, `text.lang` 
asks for the current language setting. In its simplest form, the `context` 
keyword refers to "right here":

```example
#set text(lang: "de")
// query the language setting "here"
#context text.lang
```

Note that calling `#text.lang` directly would be an error, because the request
cannot be answered without knowledge of the context. The field names supported 
by a given element function always correspond to the named parameters documented 
on each element's page.

Moreover, some functions, such as [`to-absolute`]($length.to-absolute) 
and [`counter.display`]($counter.display) are only applicable in a context, 
because their results depend on the current settings of style properties. 
When another function `foo()` calls a context-dependent function, it becomes 
itself context-dependent:

```example
#let foo() = 1em.to-absolute()
#context {
  // foo() cannot be called outside of a context
  foo() == text.size
}
```

When a property is changed, the response to the query changes accordingly: 

```example
#set text(lang: "en")
#context text.lang

#set text(lang: "de")
#context text.lang
```

The output of a `#context ...` call is static in the form of opaque 
`content`. Write access to context output is prohibited, as it would 
often result in invalid code: If the context changes between read and 
write, overwriting a property would cause an inconsistent system state. 
In fact, context-dependent property fields are immutable constants even 
within the context itself:

```example
#set text(lang: "en")
#context [
  call 1: #text.lang \

  #set text(lang: "fr")
  call 2: #text.lang
]
```

Both calls have the same output 'en', because `text.lang` is assigned
upon entry in the context and remains constant until the end of its scope 
(the closing `]`). It does not "see" the `#set text(lang: "fr")` before 
call 2. Compare this to the previous example: there we got two different 
results because we created two different contexts.

However, immutability only applies to the property fields themselves. 
Content creation instructions within a context _do_ see the effect of 
the set rule. Consider the same example with font size:

```example
#set text(size: 50pt)
#context [
  call 1: #text.size \

  #set text(size: 25pt)
  call 2: #text.size
]
```

Call 2 still outputs '50pt', because `text.size` is a constant.
However, this output is printed in '25pt' font, as specified by the set
rule before the call. This illustrates the importance of picking the 
right insertion point for a context to get access to precisely the right 
styles. If you need access to updated property fields after a set rule, 
you can use nested contexts:

```example
#set text(lang: "en")
#context [
  call 1: #text.lang \

  #set text(lang: "fr")
  call 2: #context text.lang
]
```

All of the above applies to `show` rules analogously, for example:

```example
#let template(body) = {
  set text(size: 25pt)
  body
}

#set text(size: 50pt)
#context [
  call 1: #text.size \

  #show: template
  call 2: #text.size \
  call 3: #context text.size
]
```

## Controlling content creation within a context

The main purpose of retrieving the current values of properties is, 
of course, to use them in the calculation of derived properties, 
instead of setting those properties manually. For example, you can
double the font size like this:

```example
#context {
  // the context allows you to
  // retrieve the current text.size
  set text(size: text.size * 200%)
  [large text \ ] 
}
original size
```

Since set rules are only active until the end of the enclosing scope, 
'original size' is printed with the original font size.
The above example is equivalent to 

```example
#{
  set text(size: 2em)
  [large text \ ]
}
original size
```

but convenient alternatives like this do not exist for most properties.
This makes contexts a powerful and versatile concept. For example, 
to double the spacing between the lines of an equation block, you can 
use the same resizing technique in a show rule. In this case, explicitly
adding the `context` keyword is not necessary, because a show rule
establishes a context automatically:

```example
#let spaced-eq(spacing: 100%, body) = {
  show math.equation.where(block: true): it => {
    // access current par.leading in the
    // context of the show rule
    set par(leading: par.leading * spacing)
    it
  }
  body
}

normal spacing:
$
x \
x
$
doubled spacing:
#spaced-eq(spacing: 200%)[$
z \
z
$]
```

## Location context
We've already seen that context gives us access to set rule values. But it can
do more: It also lets us know _where_ in the document we currently are, relative
to other elements, and absolutely on the pages. We can use this information to
create very flexible interactions between different document parts. This
underpins features like heading numbering, the table of contents, or page
headers dependent on section headings.

Some functions like [`counter.get`]($counter.get) implicitly access the current
location. In the example below, we want to retrieve the value of the heading
counter. Since it changes throughout the document, we need to first enter a
context expression. Then, we use `get` to retrieve the counter's current value.
This function accesses the current location from the context to resolve the
counter value. Counters have multiple levels and `get` returns an array with the
resolved numbers. Thus, we get the following result:

```example
#set heading(numbering: "1.")

= Introduction
#lorem(5)

#context counter(heading).get()

= Background
#lorem(5)

#context counter(heading).get()
```

For more flexibility, we can also use the [`here`] function to directly extract
the current [location] from the context. The example below
demonstrates this:

- We first have `{counter(heading).get()}`, which resolves to `{(2,)}` as
  before.
- We then use the more powerful  [`counter.at`] with [`here`], which in
  combination is equivalent to `get`, and thus get `{(2,)}`.
- Finally, we use `at` with a [label] to retrieve the value of the counter at a
  _different_ location in the document, in our case that of the introduction
  heading. This yields `{(1,)}`. Typst's context system gives us time travel
  abilities and lets us retrieve the values of any counters and states at _any_
  location in the document.

```example
#set heading(numbering: "1.")

= Introduction <intro>
#lorem(5)

= Background <back>
#lorem(5)

#context [
  #counter(heading).get() \
  #counter(heading).at(here()) \
  #counter(heading).at(<intro>)
]
```

The rule that context-dependent variables and functions remain constant 
within a given `context` also applies to location context. The function
`counter.display()` is an example for this behavior. Below, call A will 
access the counter's value upon _entry_ into the context, i.e. '1' - it 
cannot see the effect of `{c.update(2)}`. In contrast, call B accesses 
the counter in a nested context and will thus see the updated value.

```example
#let c = counter("mycounter")
#c.update(1)
#context [
  #c.update(2)
  call A: #c.display() \
  call B: #context c.display()
]
```

As mentioned before, we can also use context to get the physical position of
elements on the pages. We do this with the [`locate`] function, which works
similarly to `counter.at`: It takes a location or other [selector] that resolves
to a unique element (could also be a label) and returns the position on the
pages for that element.

```example
Background is at: \
#context locate(<back>).position()

= Introduction <intro>
#lorem(5)
#pagebreak()

= Background <back>
#lorem(5)
```

There are other functions that make use of the location context, most
prominently [`query`]. Take a look at the
[introspection]($category/introspection) category for more details on those.

## Compiler iterations
To resolve contextual interactions, the Typst compiler processes your document
multiple times. For instance, to resolve a `locate` call, Typst first provides a
placeholder position, layouts your document and then recompiles with the known
position from the finished layout. The same approach is taken to resolve
counters, states, and queries. In certain cases, Typst may even need more than
two iterations to resolve everything. While that's sometimes a necessity, it may
also be a sign of misuse of contextual functions (e.g. of
[state]($state/#caution)). If Typst cannot resolve everything within five
attempts, it will stop and output the warning "layout did not converge within 5
attempts."

A very careful reader might have noticed that not all of the functions presented
above actually make use of the current location. While
`{counter(heading).get()}` definitely depends on it,
`{counter(heading).at(<intro>)}`, for instance, does not. However, it still
requires context. While its value is always the same _within_ one compilation
iteration, it may change over the course of multiple compiler iterations. If one
could call it directly at the top level of a module, the whole module and its
exports could change over the course of multiple compiler iterations, which
would not be desirable.

[^1]: Currently, all show rules provide styling context, but only show rules on
      [locatable]($location/#locatable) elements provide a location context.
