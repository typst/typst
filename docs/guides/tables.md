---
description: |
  Not sure how to change table strokes? Need to rotate a table? This guide
  explains all you need to know about tables in Typst.
---

# Table guide { # }

Tables are a great way to present data to your readers in an easily readable,
compact, and organized manner. They are not only used for numerical values, but
also survery responses, task planning, schedules, and more. Because of this wide
set of possible applications, there is no single best way to lay out a table.
Instead, think about how your table can best serve your readers given its
structure, the data you want to highlight, and your document's design.

Typst can help you with your tables by automatting styling, importing data from
other applications, and more! This guide takes you through a few of the most
common questions you may have when adding a table to your document with Typst.
Feel free to skip to the section most relevant to you – we designed this guide
to be read out of order.

If you want to look up a detail of how tables work, you should also [check out
their reference page]($table). And if you are looking for a table of contents,
the reference page of the [`outline` function]($outline) is the right place to
learn more.

## How do I create a basic table? { #basic-tables }

In order to create a table in Typst, use the [`table` function]($table). For a
basic table, you need to tell the table function two things:

- The number columns
- The content for each of the table cells

So let's say, you want to create a table with two columns describing the
ingredients for a cookie recipe:

```example
#table(
  columns: 2,
  [*Amount*], [*Ingredient*],
  [360g], [Baking flour],
  [250g], [Butter (room temperature)],
  [150g], [Brown sugar],
  [100g], [Cane sugar],
  [100g], [70% cocoa chocolate],
  [100g], [35-40% cocoa chocolate],
  [2], [Eggs],
  [Pinch], [Salt],
  [Drizzle], [Vanilla extract],
)
```

This example shows how to call, configure, and populate a table. Both the column
count and cell contents are passed to the table as arguments. The [argument
list]($function) is surrounded by round parentheses. In it, we first pass the
column count as a named argument. Then, we pass multiple [content
blocks]($content) as positional arguments. Each content block contains the
contents for a single cell.

To make the example more legible, we have placed two content block arguments on
each line, mimicking how they would appear in the table. You could also write
each cell on its own line. Typst does not care on which line you place the
arguments. Instead, Typst will place the content cells from left to right (or
right to left, if that is the writing direction of your language) and then from
top to bottom. It will automatically add enough rows to your table so that it
fits all of your content.

It is best to wrap the header row of your table in the (`table.header`
function)[$table.header]. This way, you or your template can automatically style
the header of all your tables. The header function will also allow future
versions of Typst to make the output more accessible to users with a
screenreader:

```example
#table(
  columns: 2,
  table.header[*Amount*][*Ingredient*],
  [360g], [Baking flour],
<<<  // ... the remaining cells
>>>  [250g], [Butter (room temperature)],
>>>  [150g], [Brown sugar],
>>>  [100g], [Cane sugar],
>>>  [100g], [70% cocoa chocolate],
>>>  [100g], [35-40% cocoa chocolate],
>>>  [2], [Eggs],
>>>  [Pinch], [Salt],
>>>  [Drizzle], [Vanilla extract],
)
```

You could also write a show rule that automatically [strongly
emphasizes]($strong) the contents of the first cells for all tables. This
quickly becomes useful if your document contains multiple tables!

```example
#show table.cell.where(y: 0): strong

#table(
  columns: 2,
  table.header[Amount][Ingredient],
  [360g], [Baking flour],
<<<  // ... the remaining cells
>>>  [250g], [Butter (room temperature)],
>>>  [150g], [Brown sugar],
>>>  [100g], [Cane sugar],
>>>  [100g], [70% cocoa chocolate],
>>>  [100g], [35-40% cocoa chocolate],
>>>  [2], [Eggs],
>>>  [Pinch], [Salt],
>>>  [Drizzle], [Vanilla extract],
)
```

We are using a show rule with a selector for cell coordinates here instead of
applying our styles directly to `table.header`. This is due to a current
limitation of Typst that will be fixed in a future release.

Congratulations, you have created your first table! Now you can proceed to
[change column sizes](#column-sizes), [adjust the strokes](#strokes), [add
striped rows](#striped-rows-and-columns), and more!

## How to change the column sizes? { #column-sizes }

If you create a table and specify the number of columns, Typst will make each
column large enough to fit its largest cell. Often, you want something
different, for example, to make a table span the whole width of the page. You
can provide a list of how wide you want each column to be through the `columns`
argument. There are a few different ways to specify column widths:

- First, there is `{auto}`. This is the default behavior and tells Typst to grow
  the column to fit its content. If there is not enough space, Typst will try
  its best to distribute the space among the `{auto}`-sized columns.
- [Lengths]($length) like `{6cm}`, `{0.7in}`, or `{120pt}`. As usual,
  you can also use a font-dependent `em` unit. This is a multiple of your
  current font size. This is useful if you want to size your table so that
  it always fits about the same amount of text, independant of font size.
- A [relative length in percent]($relative) such as `{40%}`. This will make the
  column take up `{40%}` of the total horizontal space available to the table,
  so either the inner width of the page or the table's container. Be mindful
  that even if you specify a list of column widths that sum up to 100%, your
  table could still become larger than its container. This is because there can
  be [gutter]($table.gutter) between columns that is not included in the column
  widths. If you want to make a table fill a page, the next option is often very
  useful.
- A [fractional part of the free space]($fraction) using the `fr` unit, such as
  `1fr`. This unit allows you to distribute the available space to columns. It
  works as follows: First, Typst sums up the lengths of all columns that do not
  use `fr`s. Then, it determines how much horizontal space is left. This
  horizontal space then gets distributed to all columns denominated in `fr`s.
  During this process, a `2fr` column will become twice as wide as a `1fr`
  column. This is where the name comes from: The width of the column is its
  fraction of the total fractionally sized columns.

Let's put this to use with a table that contains the dates, numbers, and
descriptions of some routine checks. The first two columns are `auto`-sized and
the last column is `1fr` wide as to fill the whole page.

```example
#table(
  columns: (auto, auto, 1fr),
  table.header[Date][°No][Description],
  [24/01/03], [813], [Filtered participant pool],
  [24/01/03], [477], [Transitioned to sec. regimen],
  [24/01/11], [051], [Cycled treatment substrate],
)
```

Here, we have passed our list of column lengths as an [array]($array), enclosed
in round parentheses, with its elements seperated by commas. The first two
columns are automatically sized, so that they take on the size of their
content and the third column is sized as `1fr` so that it fills up the
remainder of the space on the page. If you wanted to instead change the second
column to be a bit more spacious, you could replace its entry in the `columns`
array with a value like `6em`.

## How to adjust the table strokes? { #strokes }

By default, Typst adds strokes between each row and column of a table. You can
adjust these strokes in a variety of ways. Which one is the most practical
depends on the modification you want to make and your intent:

- Do you want to style all tables in your document, irrespective of their size
  and content? Use the `table` function's [stroke]($table.stroke) argument in a
  set rule or use a show-set rule on [`table.cell`'s stroke
  argument]($table.cell.stroke).
- Do you want to customize all lines in a single table? Use the `table`
  function's [stroke]($table.stroke) argument when calling the table function.
- Do you want to change, add, or remove the stroke around a single cell? Use
  the `table.cell` element in the argument list of your table call.
- Do you want to change, add, or remove a single horizontal or vertical stroke
  in a single table? Use the [`table.hline`]($table.hline) and
  [`table.vline`]($table.vline) elements in the argument list of your table
  call.

We will go over all of these options with examples next! First, we will tackle
the `table` function's [stroke]($table.stroke) argument. Here, you can adjust
both how the table's lines get drawn and configure which lines are drawn at all.

Let's start by modifying the color and thickness of the stroke:

```example
#table(
  columns: 4,
  stroke: 0.5pt + rgb("666675"),
  [*Monday*], [11.5], [13.0], [4.0],
  [*Tuesday*], [8.0], [14.5], [5.0],
  [*Wednesday*], [9.0], [18.5], [13.0],
)
```

This makes the table lines a bit less wide and uses a bluish gray. You can see
that we added a width in point to a color to achieve our customized stroke. This
addition yields a value of the [_stroke type_]($stroke). Alternatively, you
can use the dictionary representation for strokes which allows you to access
advanced features such as dashed lines.

The previous example showed how to use the stroke argument in the table
function's invocation. Alternatively, you can specify the stroke argument in the
`table`'s set rule. This will have exactly the same effect on all subsequent
`table` calls as if the stroke argument was specified in the argument list. This
is useful if you are writing a template or want to style your whole document.

```typ
// Renders the exact same as the last example
#set table(stroke: 0.5pt + rgb("666675"))

#table(
  columns: 4,
  [*Monday*], [11.5], [13.0], [4.0],
  [*Tuesday*], [8.0], [14.5], [5.0],
  [*Wednesday*], [9.0], [18.5], [13.0],
)
```

For small tables, you sometimes want to suppress all strokes because they add
too much visual noise. To do this, just set the stroke argument to `none`:

```example
#table(
  columns: 4,
  stroke: none,
  [*Monday*], [11.5], [13.0], [4.0],
  [*Tuesday*], [8.0], [14.5], [5.0],
  [*Wednesday*], [9.0], [18.5], [13.0],
)
```

If you want more fine-grained control of where lines get placed in your table,
you can also pass a dictionary with the keys `top`, `left`, `right`, `bottom`
(controlling the respective cell sides), `x`, `y` (controlling vertical and
horizontal strokes), and `rest` (covers all strokes not styled by other
dictionary entries). All keys are optional; omitted keys will be treated as if
their value was the default value. For example, to get a table with only
horizontal lines, you can do this:

```example
#table(
  columns: 2,
  stroke: (x: none),
  [☒], [Close cabin door],
  [☐], [Start engines],
  [☐], [Radio tower],
  [☐], [Push back],
)
```

This turns off all vertical strokes and leaves the horizontal strokes in place.
To achieve the reverse effect (only horizontal strokes), set the stroke argument
to `(y: none)` instead.

If you want to excert more control, for example to draw only the first
horizontal line or omit the outer lines, you can specify a function here
instead. The function should return a stroke given the zero-indexed x and y
position of the current cell. You should only need these functions if you are a
template author, do not use a template, or need to heavily customize your
tables. Otherwise, your template should set appropriate default table strokes.

For example, this is a set rule that draws all horizontal lines except for the
very first and last line.

```example
#show table.cell.where(y: 0): strong
#set table(stroke: (_, y) => if y > 0 { (top: 0.8pt) })

#table(
  columns: 3,
  table.header[Technique][Advantage][Drawback],
  [Diegetic], [Immersive], [May be contrived],
  [Extradiegetic], [Breaks immersion], [Obstrusive],
  [Omitted], [Fosters engagement], [May fracture audience],
)
```

In the set rule, we pass a function that receives two arguments, assigning the
vertical coordinate to `y` and discarding the horizontal coordinate. It then
returns a stroke dictionary with a `0.8pt` top stroke for all but the first
line. The cells in the first line instead implicitly receive `none` as the
return value. You can easily modify this function to just draw the inner
vertical lines instead as `{(x, _) => if x > 0 { (left: 0.8pt) }}`.

Let's try a few more stroking functions. The next function will only draw a line
below the first row:

```example
>>> #show table.cell.where(y: 0): strong
#set table(stroke: (_, y) = if y == 0 { (bottom: 1pt) })

<<< // Table as seen above
>>> #table(
>>>   columns: 3,
>>>   table.header[Technique][Advantage][Drawback],
>>>   [Diegetic], [Immersive], [May be contrived],
>>>   [Extradiegetic], [Breaks immersion], [Obstrusive],
>>>   [Omitted], [Fosters engagement], [May fracture audience],
>>> )
```

If you understood the first example, it becomes obvious what happens here. We
check if we are in the first row. If so, we return a bottom stroke. Otherwise,
we'll return `none` implicitly.

The next example shows how to draw all but the outer lines:

```example
>>> #show table.cell.where(y: 0): strong
#set table(stroke: (x, y) => (
  left: if x > 0 { .8pt },
  top: if y > 0 { .8pt },
))

<<< // Table as seen above
>>> #table(
>>>   columns: 3,
>>>   table.header[Technique][Advantage][Drawback],
>>>   [Diegetic], [Immersive], [May be contrived],
>>>   [Extradiegetic], [Breaks immersion], [Obstrusive],
>>>   [Omitted], [Fosters engagement], [May fracture audience],
>>> )
```

This example uses both the `x` and `y` coordinates. It omits the left stroke in
the first column and the top stroke in the first row. The right and bottom lines
are not drawn.

Finally, here is a table that draws all lines except for the vertical lines in
the first row. It looks a bit like a calendar.

```example
>>> #show table.cell.where(y: 0): strong
#set table(stroke: (x, y) => (
  left: if x == 0 or y > 0 { 1pt } else { 0pt },
  right: 1pt,
  top: if y < 2 { 1pt } else { 0pt },
  bottom: 1pt,
))

<<< // Table as seen above
>>> #table(
>>>   columns: 3,
>>>   table.header[Technique][Advantage][Drawback],
>>>   [Diegetic], [Immersive], [May be contrived],
>>>   [Extradiegetic], [Breaks immersion], [Obstrusive],
>>>   [Omitted], [Fosters engagement], [May fracture audience],
>>> )
```

This example is a bit more complex. We start by drawing all the strokes on the
right of the cells. But this means that we have drawn strokes in the top row too
that we don't need! We use the fact that `left` will override `right` and only
draw the left line if we are not in the first row or if we are in the first
column. In all other cases, we explicitly remove the left line. Finally, we draw
the horizontal lines by first setting the bottom line and then for the first two
rows with the `top` key, suppressing all other top lines. The last line appears
because there is no `top` line that could suppress it.

### How to achieve a double line? { #double-stroke }

## How to import data into a table? { #import-data }

## How to rotate a table? { #rotate-table }

## How to merge cells? { #merge-cells }

## How to get a striped table? { #striped-rows-and-columns }

## How do I caption and reference my table? { #captions-and-references }

## How to break a table across pages? { #pagebreaks }

## How to align the contents of the cells in my table? { #alignment }

## What if I need a the table function for something that isn't a table? { #grid }
