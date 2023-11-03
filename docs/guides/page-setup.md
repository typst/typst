---
description: |
  An in-depth guide to setting page dimensions, margins, and page numbers in
  Typst. Learn how to create appealing and clear layouts and get there quickly.
---

# Page setup guide
Your page setup is a big part of the first impression your document gives. Line
lengths, margins, and columns influence
[appearance](https://practicaltypography.com/page-margins.html) and
[legibility](https://designregression.com/article/line-length-revisited-following-the-research)
while the right headers and footers will help your reader easily navigate your
document. This guide will help you to customize pages, margins, headers,
footers, and page numbers so that they are the right fit for your content and
you can get started with writing.

In Typst, each page has a width, a height, and margins on all four sides. The
top and bottom margins may contain a header and footer. The set rule of the
[`{page}`]($page) element is where you control all of the page setup. If you
make changes with this set rule, Typst will ensure that there is a new and
conforming empty page afterward, so it may insert a page break. Therefore, it is
best to specify your [`{page}`]($page) set rule at the start of your document or
in your template.

```example
#set rect(
  width: 100%,
  height: 100%,
  inset: 4pt,
)
>>> #set text(6pt)
>>> #set page(margin: auto)

#set page(
  paper: "iso-b7",
  header: rect(fill: aqua)[Header],
  footer: rect(fill: aqua)[Footer],
  number-align: center,
)

#rect(fill: aqua)
```

This example visualizes the dimensions for page content, headers, and footers.
The page content is the page size (ISO B7) minus each side's default margin. In
the top and the bottom margin, there are stroked rectangles visualizing the
header and footer. They do not touch the main content, instead, they are offset
by 30% of the respective margin. You can control this offset by specifying the
[`header-ascent`]($page.header-ascent) and
[`footer-descent`]($page.footer-descent) arguments.

Below, the guide will go more into detail on how to accomplish common page setup
requirements with examples.

## Customize page size and margins { #customize-margins }
Typst's default page size is A4 paper. Depending on your region and your use
case, you will want to change this. You can do this by using the
[`{page}`]($page) set rule and passing it a string argument to use a common page
size. Options include the complete ISO 216 series (e.g. `"iso-a4"`, `"iso-c2"`),
customary US formats like `"us-legal"` or `"us-letter"`, and more. Check out the
reference for the [page's paper argument]($page.paper) to learn about all
available options.

```example
>>> #set page(margin: auto)
#set page("us-letter")

This page likes freedom.
```

If you need to customize your page size to some dimensions, you can specify the
named arguments [`width`]($page.width) and [`height`]($page.height) instead.

```example
>>> #set page(margin: auto)
#set page(width: 12cm, height: 12cm)

This page is a square.
```

### Change the page's margins { #change-margins }
Margins are a vital ingredient for good typography:
[Typographers consider lines that fit between 45 and 75 characters best length
for legibility](http://webtypography.net/2.1.2) and your margins and
[columns](#columns) help define line widths. By default, Typst will create
margins proportional to the page size of your document. To set custom margins,
you will use the [`margin`]($page.margin) argument in the [`{page}`]($page) set
rule.

The `margin` argument will accept a length if you want to set all margins to the
same width. However, you often want to set different margins on each side. To do
this, you can pass a dictionary:

```example
#set page(margin: (
  top: 3cm,
  bottom: 2cm,
  x: 1.5cm,
))

#lorem(100)
```

The page margin dictionary can have keys for each side (`top`, `bottom`, `left`,
`right`), but you can also control left and right together by setting the `x`
key of the margin dictionary, like in the example. Likewise, the top and bottom
margins can be adjusted together by setting the `y` key.

If you do not specify margins for all sides in the margin dictionary, the old
margins will remain in effect for the unset sides. To prevent this and set all
remaining margins to a common size, you can use the `rest` key. For example,
`[#set page(margin: (left: 1.5in, rest: 1in))]` will set the left margin to 1.5
inches and the remaining margins to one inch.

### Different margins on alternating pages { #alternating-margins }
Sometimes, you'll need to alternate horizontal margins for even and odd pages,
for example, to have more room towards the spine of a book than on the outsides
of its pages. Typst keeps track of whether a page is to the left or right of the
binding. You can use this information and set the `inside` or `outside` keys of
the margin dictionary. The `inside` margin points towards the spine, and the
`outside` margin points towards the edge of the bound book.

```typ
#set page(margin: (inside: 2.5cm, outside: 2cm, y: 1.75cm))
```

Typst will assume that documents written in Left-to-Right scripts are bound on
the left while books written in Right-to-Left scripts are bound on the right.
However, you will need to change this in some cases: If your first page is
output by a different app, the binding is reversed from Typst's perspective.
Also, some books, like English-language Mangas are customarily bound on the
right, despite English using Left-to-Right script. To change the binding side
and explicitly set where the `inside` and `outside` are, set the
[`binding`]($page.binding) argument in the [`{page}`]($page) set rule.

```typ
// Produce a book bound on the right,
// even though it is set in Spanish.
#set text(lang: "es")
#set page(binding: right)
```

If `binding` is `left`, `inside` margins will be on the left on odd pages, and
vice versa.

## Add headers and footers { #headers-and-footers }
Headers and footers are inserted in the top and bottom margins of every page.
You can add custom headers and footers or just insert a page number.

In case you need more than just a page number, the best way to insert a header
and a footer are the [`header`]($page.header) and [`footer`]($page.footer)
arguments of the [`{page}`]($page) set rule. You can pass any content as their
values:

```example
>>> #set page("a5", margin: (x: 2.5cm, y: 3cm))
#set page(header: [
  _Lisa Strassner's Thesis_
  #h(1fr)
  National Academy of Sciences
])

#lorem(150)
```

Headers are bottom-aligned by default so that they do not collide with the top
edge of the page. You can change this by wrapping your header in the
[`{align}`]($align) function.

### Different header and footer on specific pages { #specific-pages }
You'll need different headers and footers on some pages. For example, you may
not want a header and footer on the title page. The example below shows how to
conditionally remove the header on the first page:

```typ
>>> #set page("a5", margin: (x: 2.5cm, y: 3cm))
#set page(header: locate(loc => {
  if counter(page).at(loc).first() > 1 [
    _Lisa Strassner's Thesis_
    #h(1fr)
    National Academy of Sciences
  ]
}))

#lorem(150)
```

This example may look intimidating, but let's break it down: We are telling
Typst that the header depends on the current [location]($locate). The `loc`
value allows other functions to find out where on the page we currently are. We
then ask Typst if the page [counter]($counter) is larger than one at our current
position. The page counter starts at one, so we are skipping the header on a
single page. Counters may have multiple levels. This feature is used for items
like headings, but the page counter will always have a single level, so we can
just look at the first one.

You can, of course, add an `else` to this example to add a different header to
the first page instead.

### Adapt headers and footers on pages with specific elements { #specific-elements }
The technique described in the previous section can be adapted to perform more
advanced tasks using Typst's labels. For example, pages with big tables could
omit their headers to help keep clutter down. We will mark our tables with a
`<big-table>` [label]($label) and use the [query system]($query) to find out if
such a label exists on the current page:

```typ
>>> #set page("a5", margin: (x: 2.5cm, y: 3cm))
#set page(header: locate(loc => {
  let page-counter = counter(page)
  let matches = query(<big-table>, loc)
  let current = page-counter.at(loc)
  let has-table = matches.any(m =>
    page-counter.at(m.location()) == current
  )

  if not has-table [
    _Lisa Strassner's Thesis_
    #h(1fr)
    National Academy of Sciences
  ]
}))

#lorem(100)
#pagebreak()

#table(
  columns: 2 * (1fr,),
  [A], [B],
  [C], [D],
) <big-table>
```

Here, we query for all instances of the `<big-table>` label. We then check that
none of the tables are on the page at our current position. If so, we print the
header. This example also uses variables to be more concise. Just as above, you
could add an `else` to add another header instead of deleting it.

## Add and customize page numbers { #page-numbers }
Page numbers help readers keep track of and reference your document more easily.
The simplest way to insert page numbers is the [`numbering`]($page.numbering)
argument of the [`{page}`]($page) set rule. You can pass a
[_numbering pattern_]($numbering.numbering) string that shows how you want your
pages to be numbered.

```example
>>> #set page("iso-b6", margin: 1.75cm)
#set page(numbering: "1")

This is a numbered page.
```

Above, you can check out the simplest conceivable example. It adds a single
Arabic page number at the center of the footer. You can specify other characters
than `"1"` to get other numerals. For example, `"i"` will yield lowercase Roman
numerals. Any character that is not interpreted as a number will be output
as-is. For example, put dashes around your page number by typing this:

```example
>>> #set page("iso-b6", margin: 1.75cm)
#set page(numbering: "— 1 —")

This is a — numbered — page.
```

You can add the total number of pages by entering a second number character in
the string.

```example
>>> #set page("iso-b6", margin: 1.75cm)
#set page(numbering: "1 of 1")

This is one of many numbered pages.
```

Go to the [`{numbering}` function reference]($numbering.numbering) to learn more
about the arguments you can pass here.

In case you need to right- or left-align the page number, use the
[`number-align`]($page.number-align) argument of the [`{page}`]($page) set rule.
Alternating alignment between even and odd pages is not currently supported
using this property. To do this, you'll need to specify a custom footer with
your footnote and query the page counter as described in the section on
conditionally omitting headers and footers.

### Custom footer with page numbers
Sometimes, you need to add other content than a page number to your footer.
However, once a footer is specified, the [`numbering`]($page.numbering) argument
of the [`{page}`]($page) set rule is ignored. This section shows you how to add
a custom footer with page numbers and more.

```example
>>> #set page("iso-b6", margin: 1.75cm)
#set page(footer: [
  *American Society of Proceedings*
  #h(1fr)
  #counter(page).display(
    "1/1",
    both: true,
  )
])

This page has a custom footer.
```

First, we add some strongly emphasized text on the left and add free space to
fill the line. Then, we call `counter(page)` to retrieve the page counter and
use its `display` function to show its current value. We also set `both` to
`{true}` so that our numbering pattern applies to the current _and_ final page
number.

We can also get more creative with the page number. For example, let's insert a
circle for each page.

```example
>>> #set page("iso-b6", margin: 1.75cm)
#set page(footer: [
  *Fun Typography Club*
  #h(1fr)
  #counter(page).display(num => {
    let circles = num * (
      box(circle(
        radius: 2pt,
        fill: navy,
      )),
    )
    box(
      inset: (bottom: 1pt),
      circles.join(h(1pt))
    )
  })
])

This page has a custom footer.
```

In this example, we use the number of pages to create an array of
[circles]($circle). The circles are wrapped in a [box]($box) so they can all
appear on the same line because they are blocks and would otherwise create
paragraph breaks. The length of this [array]($array) depends on the current page
number.

We then insert the circles at the right side of the footer, with 1pt of space
between them. The join method of an array will attempt to
[_join_]($scripting/#blocks) the different values of an array into a single
value, interspersed with its argument. In our case, we get a single content
value with circles and spaces between them that we can use with the align
function. Finally, we use another box to ensure that the text and the circles
can share a line and use the [`inset` argument]($box.inset) to raise the circles
a bit so they line up nicely with the text.

### Reset the page number and skip pages { #skip-pages }
Do you, at some point in your document, need to reset the page number? Maybe you
want to start with the first page only after the title page. Or maybe you need
to skip a few page numbers because you will insert pages into the final printed
product.

The right way to modify the page number is to manipulate the page
[counter]($counter). The simplest manipulation is to set the counter back to 1.

```typ
#counter(page).update(1)
```

This line will reset the page counter back to one. It should be placed at the
start of a page because it will otherwise create a page break. You can also
update the counter given its previous value by passing a function:

```typ
#counter(page).update(n => n + 5)
```

In this example, we skip five pages. `n` is the current value of the page
counter and `n + 5` is the return value of our function.

In case you need to retrieve the actual page number instead of the value of the
page counter, you can use the [`page`]($locate) method on the argument of the
`{locate}` closure:

```example
#counter(page).update(n => n + 5)

// This returns one even though the
// page counter was incremented by 5.
#locate(loc => loc.page())
```

You can also obtain the page numbering pattern from the `{locate}` closure
parameter with the [`page-numbering`]($locate) method.

## Add columns { #columns }
Add columns to your document to fit more on a page while maintaining legible
line lengths. Columns are vertical blocks of text which are separated by some
whitespace. This space is called the gutter.

If all of your content needs to be laid out in columns, you can just specify the
desired number of columns in the [`{page}`]($page.columns) set rule:

```example
>>> #set page(height: 120pt)
#set page(columns: 2)
#lorem(30)
```

If you need to adjust the gutter between the columns, refer to the method used
in the next section.

### Use columns anywhere in your document { #columns-anywhere }
Very commonly, scientific papers have a single-column title and abstract, while
the main body is set in two-columns. To achieve this effect, Typst includes a
standalone [`{columns}` function]($columns) that can be used to insert columns
anywhere on a page.

Conceptually, the `columns` function must wrap the content of the columns:

```example:single
>>> #set page(height: 180pt)
= Impacts of Odobenidae

#set par(justify: true)
>>> #h(11pt)
#columns(2)[
  == About seals in the wild
  #lorem(80)
]
```

However, we can use the ["everything show rule"]($styling/#show-rules) to reduce
nesting and write more legible Typst markup:

```example:single
>>> #set page(height: 180pt)
= Impacts of Odobenidae

#set par(justify: true)
>>> #h(11pt)
#show: columns.with(2)

== About seals in the wild
#lorem(80)
```

The show rule will wrap everything that comes after it in its function. The
[`with` mehtod]($function.with) allows us to pass arguments, in this case, the
column count, to a function without calling it.

Another use of the `columns` function is to create columns inside of a container
like a rectangle or to customize gutter size:

```example
#rect(
  width: 6cm,
  height: 3.5cm,
  columns(2, gutter: 12pt)[
    In the dimly lit gas station,
    a solitary taxi stood silently,
    its yellow paint fading with
    time. Its windows were dark,
    its engine idle, and its tires
    rested on the cold concrete.
  ]
)
```

### Balanced columns
If the columns on the last page of a document differ greatly in length, they may
create a lopsided and unappealing layout. That's why typographers will often
equalize the length of columns on the last page. This effect is called balancing
columns. Typst cannot yet balance columns automatically. However, you can
balance columns manually by placing [`[#colbreak()]`]($colbreak) at an
appropriate spot in your markup, creating the desired column break manually.


## One-off modifications
You do not need to override your page settings if you need to insert a single
page with a different setup. For example, you may want to insert a page that's
flipped to landscape to insert a big table or change the margin and columns for
your title page. In this case, you can call [`{page}`]($page) as a function with
your content as an argument and the overrides as the other arguments. This will
insert enough new pages with your overridden settings to place your content on
them. Typst will revert to the page settings from the set rule after the call.

```example
>>> #set page("a6")
#page(flipped: true)[
  = Multiplication table

  #table(
    columns: 5 * (1fr,),
    ..for x in range(1, 10) {
      for y in range(1, 6) {
        (str(x*y),)
      }
    }
  )
]
```
