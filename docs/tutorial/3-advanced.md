---
description: Typst's tutorial.
---

# Advanced Styling
In the previous two chapters of this tutorial, you have learned how to write a
document in Typst and how to change its formatting. The report you wrote
throughout the last two chapters got a straight A and your supervisor wants to
base a conference paper on it! The report will of course have to comply with the
conference's style guide. Let's see how we can achieve that.

Before we start, let's create a team, invite your supervisor and add them to the
team. You can do this by going back to the app dashboard with the back icon in
the top left corner of the editor. Then, choose the plus icon in the left
toolbar and create a team. Finally, click on the new team and go to its settings
by clicking 'manage team' next to the team name. Now you can invite your
supervisor by email.

![The team settings](3-advanced-team-settings.png)

Next, move your project into the team: Open it, going to its settings by
choosing the gear icon in the left toolbar and selecting your new team from the
owners dropdown. Don't forget to save your changes!

Now, your supervisor can also edit the project and you can both see the changes
in real time. You can join our [Discord server](https://discord.gg/2uDybryKPe)
to find other users and try teams with them!

## The conference guidelines { #guidelines }
The layout guidelines are available on the conference website. Let's take a look
at them:

- The font should be an 11pt serif font
- The title should be in 17pt and bold
- The paper contains a single-column abstract and two-column main text
- The abstract should be centered
- The main text should be justified
- First level section headings should be 13pt, centered, and rendered in small
  capitals
- Second level headings are run-ins, italicized and have the same size as the
  body text
- Finally, the pages should be US letter sized, numbered in the center of the
  footer and the top right corner of each page should contain the title of the
  paper

We already know how to do many of these things, but for some of them, we'll need
to learn some new tricks.

## Writing the right set rules { #set-rules }
Let's start by writing some set rules for the document.

```example
#set page(
>>> margin: auto,
  paper: "us-letter",
  header: align(right)[
    A fluid dynamic model for
    glacier flow
  ],
  numbering: "1",
)
#set par(justify: true)
#set text(
  font: "Libertinus Serif",
  size: 11pt,
)

#lorem(600)
```

You are already familiar with most of what is going on here. We set the text
size to `{11pt}` and the font to Libertinus Serif. We also enable paragraph
justification and set the page size to US letter.

The `header` argument is new: With it, we can provide content to fill the top
margin of every page. In the header, we specify our paper's title as requested
by the conference style guide. We use the `align` function to align the text to
the right.

Last but not least is the `numbering` argument. Here, we can provide a
[numbering pattern]($numbering) that defines how to number the pages. By
setting it to `{"1"}`, Typst only displays the bare page number. Setting it to
`{"(1/1)"}` would have displayed the current page and total number of pages
surrounded by parentheses. And we could even have provided a completely custom
function here to format things to our liking.

## Creating a title and abstract { #title-and-abstract }
Now, let's add a title and an abstract. We'll start with the title. We center
align it and increase its font weight by enclosing it in `[*stars*]`.

```example
>>> #set page(width: 300pt, margin: 30pt)
>>> #set text(font: "Libertinus Serif", 11pt)
#align(center, text(17pt)[
  *A fluid dynamic model
  for glacier flow*
])
```

This looks right. We used the `text` function to override the previous text
set rule locally, increasing the size to 17pt for the function's argument. Let's
also add the author list: Since we are writing this paper together with our
supervisor, we'll add our own and their name.

```example
>>> #set page(width: 300pt, margin: 30pt)
>>> #set text(font: "Libertinus Serif", 11pt)
>>>
>>> #align(center, text(17pt)[
>>>   *A fluid dynamic model
>>>   for glacier flow*
>>> ])
#grid(
  columns: (1fr, 1fr),
  align(center)[
    Therese Tungsten \
    Artos Institute \
    #link("mailto:tung@artos.edu")
  ],
  align(center)[
    Dr. John Doe \
    Artos Institute \
    #link("mailto:doe@artos.edu")
  ]
)
```

The two author blocks are laid out next to each other. We use the [`grid`]
function to create this layout. With a grid, we can control exactly how large
each column is and which content goes into which cell. The `columns` argument
takes an array of [relative lengths]($relative) or [fractions]($fraction). In
this case, we passed it two equal fractional sizes, telling it to split the
available space into two equal columns. We then passed two content arguments to
the grid function. The first with our own details, and the second with our
supervisors'. We again use the `align` function to center the content within the
column. The grid takes an arbitrary number of content arguments specifying the
cells. Rows are added automatically, but they can also be manually sized with
the `rows` argument.

Now, let's add the abstract. Remember that the conference wants the abstract to
be set ragged and centered.

```example:0,0,612,317.5
>>> #set page(
>>>   "us-letter",
>>>   margin: auto,
>>>   header: align(right + horizon)[
>>>     A fluid dynamic model for
>>>     glacier flow
>>>   ],
>>>   numbering: "1",
>>> )
>>> #set par(justify: true)
>>> #set text(font: "Libertinus Serif", 11pt)
>>>
>>> #align(center, text(17pt)[
>>>   *A fluid dynamic model
>>>   for glacier flow*
>>> ])
>>>
>>> #grid(
>>>   columns: (1fr, 1fr),
>>>   align(center)[
>>>     Therese Tungsten \
>>>     Artos Institute \
>>>     #link("mailto:tung@artos.edu")
>>>   ],
>>>   align(center)[
>>>     Dr. John Doe \
>>>     Artos Institute \
>>>     #link("mailto:doe@artos.edu")
>>>   ]
>>> )
>>>
<<< ...

#align(center)[
  #set par(justify: false)
  *Abstract* \
  #lorem(80)
]
>>> #lorem(600)
```

Well done! One notable thing is that we used a set rule within the content
argument of `align` to turn off justification for the abstract. This does not
affect the remainder of the document even though it was specified after the
first set rule because content blocks _scope_ styling. Anything set within a
content block will only affect the content within that block.

Another tweak could be to save the paper title in a variable, so that we do not
have to type it twice, for header and title. We can do that with the `{let}`
keyword:

```example:single
#let title = [
  A fluid dynamic model
  for glacier flow
]

<<< ...

#set page(
>>> "us-letter",
>>> margin: auto,
  header: align(
    right + horizon,
    title
  ),
<<<   ...
>>> numbering: "1",
)
>>> #set par(justify: true)
>>> #set text(font: "Libertinus Serif", 11pt)

#align(center, text(17pt)[
  *#title*
])

<<< ...

>>> #grid(
>>>   columns: (1fr, 1fr),
>>>   align(center)[
>>>     Therese Tungsten \
>>>     Artos Institute \
>>>     #link("mailto:tung@artos.edu")
>>>   ],
>>>   align(center)[
>>>     Dr. John Doe \
>>>     Artos Institute \
>>>     #link("mailto:doe@artos.edu")
>>>   ]
>>> )
>>>
>>> #align(center)[
>>>   #set par(justify: false)
>>>   *Abstract* \
>>>   #lorem(80)
>>> ]
>>>
>>> #lorem(600)
```

After we bound the content to the `title` variable, we can use it in functions
and also within markup (prefixed by `#`, like functions). This way, if we decide
on another title, we can easily change it in one place.

## Adding columns and headings { #columns-and-headings }
The paper above unfortunately looks like a wall of lead. To fix that, let's add
some headings and switch our paper to a two-column layout. Fortunately, that's
easy to do: We just need to amend our `page` set rule with the `columns`
argument.

By adding `{columns: 2}` to the argument list, we have wrapped the whole
document in two columns. However, that would also affect the title and authors
overview. To keep them spanning the whole page, we can wrap them in a function
call to [`{place}`]($place). Place expects an alignment and the content it
should place as positional arguments. Using the named `{scope}` argument, we can
decide if the items should be placed relative to the current column or its
parent (the page). There is one more thing to configure: If no other arguments
are provided, `{place}` takes its content out of the flow of the document and
positions it over the other content without affecting the layout of other
content in its container:

```example
#place(
  top + center,
  rect(fill: black),
)
#lorem(30)
```

If we hadn't used `{place}` here, the square would be in its own line, but here
it overlaps the few lines of text following it. Likewise, that text acts like as
if there was no square. To change this behavior, we can pass the argument
`{float: true}` to ensure that the space taken up by the placed item at the top
or bottom of the page is not occupied by any other content.

```example:single
>>> #let title = [
>>>   A fluid dynamic model
>>>   for glacier flow
>>> ]
>>>
#set page(
>>> margin: auto,
  paper: "us-letter",
  header: align(
    right + horizon,
    title
  ),
  numbering: "1",
  columns: 2,
)
>>> #set par(justify: true)
>>> #set text(font: "Libertinus Serif", 11pt)

#place(
  top + center,
  float: true,
  scope: "parent",
  clearance: 2em,
)[
>>> #text(
>>>   17pt,
>>>   weight: "bold",
>>>   title,
>>> )
>>>
>>> #grid(
>>>   columns: (1fr, 1fr),
>>>   [
>>>     Therese Tungsten \
>>>     Artos Institute \
>>>     #link("mailto:tung@artos.edu")
>>>   ],
>>>   [
>>>     Dr. John Doe \
>>>     Artos Institute \
>>>     #link("mailto:doe@artos.edu")
>>>   ]
>>> )
<<<   ...

  #par(justify: false)[
    *Abstract* \
    #lorem(80)
  ]
]

= Introduction
#lorem(300)

= Related Work
#lorem(200)
```

In this example, we also used the `clearance` argument of the `{place}` function
to provide the space between it and the body instead of using the [`{v}`]($v)
function. We can also remove the explicit `{align(center, ..)}` calls around the
various parts since they inherit the center alignment from the placement.

Now there is only one thing left to do: Style our headings. We need to make them
centered and use small capitals. Because the `heading` function does not offer
a way to set any of that, we need to write our own heading show rule.

```example:50,250,265,270
>>> #let title = [
>>>   A fluid dynamic model
>>>   for glacier flow
>>> ]
>>>
>>> #set page(
>>>   "us-letter",
>>>   margin: auto,
>>>   header: align(
>>>     right + horizon,
>>>     title
>>>   ),
>>>   numbering: "1",
>>>   columns: 2,
>>> )
>>> #set par(justify: true)
>>> #set text(font: "Libertinus Serif", 11pt)
#show heading: it => [
  #set align(center)
  #set text(13pt, weight: "regular")
  #block(smallcaps(it.body))
]

<<< ...
>>> #place(
>>>   top + center,
>>>   float: true,
>>>   scope: "parent",
>>>   clearance: 2em,
>>> )[
>>>   #text(
>>>     17pt,
>>>     weight: "bold",
>>>     title,
>>>   )
>>>
>>>   #grid(
>>>     columns: (1fr, 1fr),
>>>     [
>>>       Therese Tungsten \
>>>       Artos Institute \
>>>       #link("mailto:tung@artos.edu")
>>>     ],
>>>     [
>>>       Dr. John Doe \
>>>       Artos Institute \
>>>       #link("mailto:doe@artos.edu")
>>>     ]
>>>   )
>>>
>>>   #par(justify: false)[
>>>     *Abstract* \
>>>     #lorem(80)
>>>   ]
>>> ]

= Introduction
<<< ...
>>> #lorem(35)

== Motivation
<<< ...
>>> #lorem(45)
```

This looks great! We used a show rule that applies to all headings. We give it a
function that gets passed the heading as a parameter. That parameter can be used
as content but it also has some fields like `title`, `numbers`, and `level` from
which we can compose a custom look. Here, we are center-aligning, setting the
font weight to `{"regular"}` because headings are bold by default, and use the
[`smallcaps`] function to render the heading's title in small capitals.

The only remaining problem is that all headings look the same now. The
"Motivation" and "Problem Statement" subsections ought to be italic run in
headers, but right now, they look indistinguishable from the section headings. We
can fix that by using a `where` selector on our set rule: This is a
[method]($scripting/#methods) we can call on headings (and other
elements) that allows us to filter them by their level. We can use it to
differentiate between section and subsection headings:

```example:50,250,265,245
>>> #let title = [
>>>   A fluid dynamic model
>>>   for glacier flow
>>> ]
>>>
>>> #set page(
>>>   "us-letter",
>>>   margin: auto,
>>>   header: align(
>>>     right + horizon,
>>>     title
>>>   ),
>>>   numbering: "1",
>>>   columns: 2,
>>> )
>>> #set par(justify: true)
>>> #set text(font: "Libertinus Serif", 11pt)
>>>
#show heading.where(
  level: 1
): it => block(width: 100%)[
  #set align(center)
  #set text(13pt, weight: "regular")
  #smallcaps(it.body)
]

#show heading.where(
  level: 2
): it => text(
  size: 11pt,
  weight: "regular",
  style: "italic",
  it.body + [.],
)
>>>
>>> #place(
>>>   top + center,
>>>   float: true,
>>>   scope: "parent",
>>>   clearance: 2em,
>>> )[
>>>   #text(
>>>     17pt,
>>>     weight: "bold",
>>>     title,
>>>   )
>>>
>>>   #grid(
>>>     columns: (1fr, 1fr),
>>>     [
>>>       Therese Tungsten \
>>>       Artos Institute \
>>>       #link("mailto:tung@artos.edu")
>>>     ],
>>>     [
>>>       Dr. John Doe \
>>>       Artos Institute \
>>>       #link("mailto:doe@artos.edu")
>>>     ]
>>>   )
>>>
>>>   #par(justify: false)[
>>>     *Abstract* \
>>>     #lorem(80)
>>>   ]
>>> ]
>>>
>>> = Introduction
>>> #lorem(35)
>>>
>>> == Motivation
>>> #lorem(45)
```

This looks great! We wrote two show rules that each selectively apply to the
first and second level headings. We used a `where` selector to filter the
headings by their level. We then rendered the subsection headings as run-ins. We
also automatically add a period to the end of the subsection headings.

Let's review the conference's style guide:
- The font should be an 11pt serif font ✓
- The title should be in 17pt and bold ✓
- The paper contains a single-column abstract and two-column main text ✓
- The abstract should be centered ✓
- The main text should be justified ✓
- First level section headings should be centered, rendered in small caps and in
  13pt ✓
- Second level headings are run-ins, italicized and have the same size as the
  body text ✓
- Finally, the pages should be US letter sized, numbered in the center and the
  top right corner of each page should contain the title of the paper ✓

We are now in compliance with all of these styles and can submit the paper to
the conference! The finished paper looks like this:

<img
  src="3-advanced-paper.png"
  alt="The finished paper"
  style="box-shadow: 0 4px 12px rgb(89 85 101 / 20%); width: 500px; max-width: 100%; display: block; margin: 24px auto;"
>

## Review
You have now learned how to create headers and footers, how to use functions and
scopes to locally override styles, how to create more complex layouts with the
[`grid`] function and how to write show rules for individual functions, and the
whole document. You also learned how to use the
[`where` selector]($styling/#show-rules) to filter the headings by their level.

The paper was a great success! You've met a lot of like-minded researchers at
the conference and are planning a project which you hope to publish at the same
venue next year. You'll need to write a new paper using the same style guide
though, so maybe now you want to create a time-saving template for you and your
team?

In the next section, we will learn how to create templates that can be reused in
multiple documents. This is a more advanced topic, so feel free to come back
to it later if you don't feel up to it right now.
