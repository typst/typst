---
description: |
  An in-depth guide to setting page dimensions, margins, and page numbers in
  Typst. Learn how to create appealing and clear layouts and get there quickly.
---

# Page setup guide
A good page setup is the basis for every legible document. This guide will help
you to set up pages, margins, headers, footers, and page numbers to your heart's
content so you can get started with writing.

In Typst, each page has a width and a height, and margins on all four sides. The
top and bottom margin may contain a header and footer. The set rule of the
`page` element is where you control all of the page setup. If you make changes
with this set rule, Typst will ensure that there is a new and conforming empty
page afterwards, so it may insert a page break. Therefore, it is best to specify
your page set rule at the start of your document or in your template.

```example
#set rect(width: 100%, height: 100%)
#set page(
  "iso-b7",
  header: rect(),
  footer: rect(),
  number-align: top + center,
)

#rect(fill: rgb("#565565"))
```

This example visualizes the dimensions for page content, headers, and footers.
The page content is the page size (ISO B7) minus each side's default margin. In
the top and the bottom margin, there are stroked rectangles visualizing header
and footer. They do not touch the main content, instead  they are offset by 30%
of the respective margin. You can control this offset by specifying
`header-ascent` and `footer-descent`.

Below, the guide will go more into detail on how to accomplish common page setup
requirements with examples.

## Customize page size and margins
Typst's default page size is A4 paper. Depending on your region and your
use-case, you will want to change this. You can do this by using the `page` set
rule and passing it a string argument to use a common page size. Options include
the complete ISO 216 series (e.g. `"iso-a4"`, `"iso-c2"`), customary US formats
like `"us-legal"` or `"us-letter"`, and more. Check out the reference for the
page size argument to learn about all available options.

```example
#set page("us-letter")

This page likes freedom.
```

If you need to customize your page size to some dimensions, you can specify the
named arguments `width` and `height` instead.

```example
#set page(width: 12cm, height: 12cm)

This page is a square.
```

### Change the page's margins
Margins are a vital ingredient for good typography: Typographers consider lines
that fit between 45 and 75 characters best length for legibility and your
margins help define line widths. By default, Typst will create margins
proportional to the page size for your document. To set custom margins, you will
use the `margin` argument in the `page` set rule.

The `margin` argument will accept a length if you want to set all margins to the
same width. However, you often want to set different margins on each side. To do
this, you can pass a dictionary:

```example
#set page(margin: (top: 3cm, bottom: 2cm, x: 1.5cm))

#lorem(100)
```

The page margin dictionary can have keys for each side (`top`, `bottom`, `left`,
`right`), but you can also control left and right together by setting the `x`
key of the margin dictionary, like in the example. Likewise, the top and bottom
margins can be adjusted together by setting the `y` key.

If you do not specify margins for all sides in the margin dictionary, the old
margins will remain in effect for the unset sides. To prevent this and set all
remaining margins to a common size, you can use the `rest` argument. For
example, `#set page(margin: (left: 1.5in, rest: 1in))` will set the left margin
to 1.5 inch and the remaining margins to one inch.

### Different margins on alternating pages
Sometimes, you'll need to alternate horizontal margins for even and odd pages,
for example to have more room towards the spine of a book than on the outsides
of its pages. Typst keeps track of whether a page is to the left or right of the
binding. You can use this information and set the `inside` or `outside` keys of
the margin dictionary. The `inside` margin points towards the spine, the
`outside` margin points towards the edge of the bound book.

```example
#set page(margin: (
    inside: 2.5cm,
    outside: 2cm,
    y: 1.75cm,
))
```

Typst will assume that documents written in Left-to-Right scripts are bound on
the left while books written in Right-to-Left scripts are bound on the right.
However, you will need to change this in some cases: If your first page is
output by a different app, the binding is reversed from Typst's perspective.
Also, some books, like English-language Mangas are customarily bound on the
right, despite English using Left-to-Right script. To change the binding side
and explicitly set where `inside` and `outside` are, set the `binding` argument
in the `page` set rule.

```example
#set text(lang: "es")

// Produce a book bound on the right,
// even though it is set in Spanish.
#set page(binding: right)
```

If `binding` is `left`, `inside` margins will be on the left on odd pages, and
vice versa.

## Add headers and footers
Headers and footers are inserted in the top and bottom margin of every page. You
can add custom headers and footers or just insert a page number.

In case you need more than just a page number, the best way to insert a header
and a footer are the `header` and `footer` arguments of the `page` set rule. You
can pass any content as their values:

```example
#set page(header: [
    _Lisa Strassner's Thesis_
    #h(1fr)
    National Academy of Sciences
])
```

Headers are bottom-aligned by default, so that they do not collide with the top
edge of the page. You can change this by wrapping your header in the `align`
function.

### Different header and footer on specific pages
You'll need different headers and footers on some pages. For example, you may
not want a header and footer on the title page. The example below shows how to
conditionally remove the header on the first page:

```example
#set page(
  header: locate(loc => {
    if (counter(page).at(loc).first() > 1) [
        _Lisa Strassner's Thesis_
        #h(1fr)
        National Academy of Sciences
    ]
  }),
)
```

This example may look intimidating, but let's break it down: We are telling
Typst that the header depends on the current location. The `loc` value allows
other functions to find out where on the page we currently are. We then ask
Typst if the page counter is larger than one at our current position. The page
counter starts at one, so we are skipping the header on a single page. Counters
may have multiple levels. This feature is used for items like headings, but the
page counter will always have a single level, so we can just look at the first
one.

You can, of course, add an `else` to this example to add a different header to
the first page instead.

### Adapt headers and footers on pages with specific elements
The technique described in the previous section can be adapted to perform more
advanced tasks using Typst's labels. For example, pages with big tables could
omit their headers to help keep clutter down. We will mark our tables with a
`<big-table>` label and use the query system to find out if such a label exists
on the current page:

```example
#set page(
  header: locate(loc => {
    let page-counter = counter(page)
    let matches = query(<big-table>, loc)
    let has-no-table = matches.all(m =>
      page-counter.at(m.location()) !=
        page-counter.at(loc)
    )

    if (has-no-table) [
        _Lisa Strassner's Thesis_
        #h(1fr)
        National Academy of Sciences
    ]
  }),
)

#lorem(100)
#pagebreak()

#table() <big-table>
```

Here, we query for all instances of the `<big-table>` label. We then check that
none of the tables are on the page at our current position. If so, we print the
header. This example also uses variables to be more concise. Just as above, you
could add an `else` to add another header instead of deleting it.

## Add and customize page numbers
Page numbers help readers keep track and reference your document more easily.
The simplest way to insert footnotes is the `numbering` argument of the page set
rule. You can pass a _numbering pattern_ string that shows how you want your
pages to be numbered.

```example
#set page(numbering: "1")
```

Above, you can check out the simplest conceivable example. It adds a single
Arabic page number at the center of the footer. You can specify other characters
than `"1"` to get other numerals. For example, `"i"` will yield lowercase Roman
numerals. Any character that is not interpreted as a number will be output
as-is. For example, put dashes around your page number by typing this:

```example
#set page(numbering: "— 1 —")
```

You can add the total number of pages by entering a second number character in
the string.

```example
#set page(numbering: "1 of 1")
```

Go to the numbering function reference to learn more about the arguments you can
pass here.

In case you need to right- or left-align the page number, use the
`numbering-align` argument of the page set rule. Alternating alignment between
even and odd pages is not currently supported using this property. To do this,
you'll need to specify a custom footer with your footnote and query the page
counter as described in the section on conditionally omitting headers and
footers.

### Custom footer with page numbers
Sometimes, you need to add other content than a page number to your footer.
However, once a footer is specified, the `numbering` argument of the `page` set
rule is ignored. This section shows you how to add a custom footer with page
numbers and more.

```example
#set page(
  footer: locate(loc => {
    let page-num = counter(page).at(loc).first()
    let page-total = counter(page).final(loc).first()

    strong[American Society of Proceedings]
    h(1fr)
    [#page-num/#page-total]
  })
)
```

The example above shows how to add a custom footer with page numbers. First of
all, we need to recover the page number using the page counter. For this, we are
using the `locate` function to check the page counter, just like in the
conditional header section. We then store the current and final page number in
variables.

Then, we can proceed to build our footer. We add a strong label on the left,
insert all the free space on the line, and finally display the current page
number and the page total. This would work just the same in the header and with
any content.

We can, of course, use the `numbering` function to use numbering pattern strings
like before:

```example
#set page(
  footer: locate(loc => {
    let page-num = counter(page).at(loc).first()
    let page-total = counter(page).final(loc).first()

    strong[American Society of Proceedings]
    h(1fr)
    numbering("i of I", page-num, page-total)
  })
)
```

The numbering function accepts multiple arguments. It will use the arguments in
order for each number character. You could, for example, put the page total in
front of the page number by reversing the argument order.

We can even use these variables to get more creative with the page number. For
example, let's insert a circle for each page.

```example
#set page(
  footer: locate(loc => {
    let page-num = counter(page).at(loc).first()
    let circles = (
        box(circle(radius: 2pt, fill: navy)),
      ) * page-num

    align(right, circles.join(h(1pt)))
  })
)
```

In this example, we use the number of pages to create an array of circles. The
circles are wrapped in a box so they can all appear on the same line, because
they are blocks and would otherwise create paragraph breaks. The length of this
array depends on the current page number.

We then insert the circles at the right side of the footer, with 1pt of space
between them. The join method of an array will attempt to _join_ the different
values of an array into a single value, interspersed with its argument. In our
case, we get a single content value with circles and spaces between them that we
can use with the align function.

### Reset the page number and skip pages
Do you, at some point in your document, reset the page number? Maybe you want to
start with the first page only after the title page. Or maybe you need to skip a
few page numbers because you will insert pages into the final printed product.

The right way to modify the page number is to manipulate the page counter. The
simplest manipulation is to set the counter back to 1.

```example
#counter(page).update(1)
```

This line will reset the page counter back to one. It should be placed at the
start of a page because it will otherwise create a page break. You can also also
update the counter given its previous value by passing a function:

```example
#counter(page).update(i => i + 5)
```

In this example, we skip five pages. `i` is the current value of the page
counter and `i + 5` is the return value of our function.

<!-- ## Add columns

### Columns after start of doc

### Balanced columns

### Marginals -->

<!-- One-off modification -->
