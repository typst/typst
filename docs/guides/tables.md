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

## How to create a basic table? { #basic-tables }

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
  [250g], [Butter (room temp.)],
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
>>>  [250g], [Butter (room temp.)],
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
>>>  [250g], [Butter (room temp.)],
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

[Further down in the guide](#stroke-functions), we cover how to use a function
in the stroke argument to customize all strokes individually. This is how you
achieve more complex stroking patterns.

### Adding individual lines in the table { #individual-strokes }

If you want to add a single horizontal or vertical line in your table, for
example to seperate a group of rows, you can use the
[`table.hline`]($table.hline) and [`table.vline`]($table.vline) elements for
horizontal and vertical lines, respectively. Add them to the argument list of
the `table` function just like you would add individual cells and a header.

Let's take a look at the example from the reference:

```example
#set table.hline(stroke: .6pt)

#table(
  stroke: none,
  columns: (auto, 1fr),
  // Morning schedule abridged.
  [14:00], [Talk: Tracked Layout],
  [15:00], [Talk: Automations],
  [16:00], [Workshop: Tables],
  table.hline(),
  [19:00], [Day 1 Attendee Mixer],
)
```

In this example, you can see that we have placed a call to `table.hline` between
the cells, producing a horizontal line at that spot. We also used a set rule on
the element to reduce its stroke width to make it fit better with the weight of
the font.

By default, Typst places horizontal and vertical lines after the current row or
column, depending on their position in the argument list. You can also manually
move them to a different position by adding the `y` (for `hline`) or `x` (for
`vline`) argument. For example, the code below would produce the same result:

```typ
#set table.hline(stroke: .6pt)

#table(
  stroke: none,
  columns: (auto, 1fr),
  // Morning schedule abridged.
  table.hline(y: 3),
  [14:00], [Talk: Tracked Layout],
  [15:00], [Talk: Automations],
  [16:00], [Workshop: Tables],
  [19:00], [Day 1 Attendee Mixer],
)
```

Let's imagine you are working with a template that shows none of the table
strokes but one between the first and second row. Now, since you also have
labels in the first column, you want to add a vertical line. However, you do not
want this vertical line to cross into the top row. You can achieve this with the
`start` argument:

```example
>>> #set page(width: 12cm)
>>> #show table.cell.where(y: 0): strong
>>> #set table(stroke: (_, y) => if y == 0 { (bottom: 1pt) })
#show table.cell.where(x: 0): smallcaps

#table(
  columns: (auto, 1fr, 1fr, 1fr),
  align: (x, _) => if x > 0 { right } else { left },

  table.vline(x: 1, start: 1),
  table.header[Trainset][Top Speed][Length][Weight],
  [TGV Réseau], [320 km/h], [200m], [383t],
  [ICE 403], [330 km/h], [201m], [409t],
  [Shinkansen N700], [300 km/h], [405m], [700t],
)
```

In this example, we have added `table.vline` at the start of our positional
argument list. But because the line is not supposed to go to the left of the
first column, we specified the `x` argument as `1`. We also set the `start`
argument to `1` so that the line does only start after the first row.

This example also contains two more things: We use the align argument with a
function not unlike those in the next subsection to right-align the data in all
but the first column and use a show-rule to make the first column of table cells
appear in small caps.

### Overriding the strokes of a single cell { #cell-stroke-override }

Imagine you want to change the stroke around a single cell. Maybe your cell is
very important and needs highlighting! For this scenario, there is the
[`table.cell` function]($table.cell). Instead of adding your content directly in
the argument list of the table, you wrap it in a `table.cell` call. Now, you can
use `table.cell`'s argument list to override the table properties, such as the
stroke, for this cell only.

Here's an example with a matrix of two of the Big Five personality factors, with
one intersection highlighted.

```example
>>> #set page(width: 16cm)
#table(
  columns: 3,
  stroke: (x: none),

  [], [*High Neuroticism*], [*Low Neuroticism*],

  [*High Agreeableness*],
  table.cell(stroke: orange + 2pt)[
    _Sensitive_ \ Prone to emotional distress but very empathetic.
  ],
  [_Compassionate_ \ Caring and stable, often seen as a supportive figure.],

  [*Low Agreeableness*],
  [_Contentious_ \ 	Competitive and easily agitated.],
  [_Detached_ \ Independent and calm, may appear aloof.],
)
```

Above, you can see that we used the `table.cell` element in the table's argument
list and passed the cell content to it. We have used the arguments to set a
wider orange stroke. Despite the fact that we disabled vertical strokes in the
table, the orange stroke appeared on all sides of the modified cell, showing
that the table's stroke configuration is discarded.

### Complex document-wide stroke customization { #stroke-functions }

This section is about customizing all lines at once in one or multiple tables.
This allows you to draw only the first horizontal line or omit the outer lines,
without knowing how many cells the table has. This is achieved by providing a
function for the table's `stroke` parameter. The function should return a stroke
given the zero-indexed x and y position of the current cell. You should only
need these functions if you are a template author, do not use a template, or
need to heavily customize your tables. Otherwise, your template should set
appropriate default table strokes.

For example, this is a set rule that draws all horizontal lines except for the
very first and last line.

```example
#show table.cell.where(x: 0): set text(style: "italic")
#show table.cell.where(y: 0): set text(style: "normal", weight: "bold")
#set table(stroke: (_, y) => if y > 0 { (top: 0.8pt) })

#table(
  columns: 3,
  align: center + horizon,
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
>>> #show table.cell: it => if it.x == 0 and it.y > 0 {
>>>   set text(style: "italic")
>>>   it
>>> } else {
>>>   it
>>> }
>>>
>>> #show table.cell.where(y: 0): strong
#set table(stroke: (_, y) => if y == 0 { (bottom: 1pt) })

<<< // Table as seen above
>>> #table(
>>>   columns: 3,
>>>   align: center + horizon,
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
>>> #show table.cell: it => if it.x == 0 and it.y > 0 {
>>>   set text(style: "italic")
>>>   it
>>> } else {
>>>   it
>>> }
>>>
>>> #show table.cell.where(y: 0): strong
#set table(stroke: (x, y) => (
  left: if x > 0 { .8pt },
  top: if y > 0 { .8pt },
))

<<< // Table as seen above
>>> #table(
>>>   columns: 3,
>>>   align: center + horizon,
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
>>> #show table.cell: it => if it.x == 0 and it.y > 0 {
>>>   set text(style: "italic")
>>>   it
>>> } else {
>>>   it
>>> }
>>>
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
>>>   align: center + horizon,
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

Typst does not yet have a native way to draw double strokes, but there are
multiple ways to emulate them, for example with [patterns]($pattern). We will
show a different workaround in this section: Table gutters.

Tables can space their cells apart using the `gutter` argument. When a gutter is
applied, a stroke is drawn on each of the now seperated cells. Now, we need to
selectively add gutter between the rows or columns for which we want to draw a
double line. The `row-gutter` and `column-gutter` arguments allow us to do this.
They accept arrays of gutter values. Let's take a look at an example:

```example
#table(
  columns: 3,
  stroke: (x: none),
  row-gutter: (2.2pt, auto),
  table.header[Date][Exercise Type][Calories Burned],
  [2023-03-15], [Swimming], [400],
  [2023-03-17], [Weightlifting], [250],
  [2023-03-18], [Yoga], [200],
)
```

We can see that we used an array for `row-gutter` that specifies a `{2.2pt}` gap
between the first and second row. It then continues with `auto` (which is the
default, in this case `{0pt}` gutter) which will be the gutter between all other
rows, since it is the last entry in the array.

## How to import data into a table? { #import-data }

Often, you need to put data that you obtained elsewhere in a table. Sometimes,
this is from Microsoft Excel or Google Sheets, sometimes this is from a dataset
on the web. Fortunately, Typst can load many [common file
formats]($category/data-loading) so you can use scripting to include their data
in a table.

The most common file format for tabular data is CSV. You can obtain a CSV file
from Excel by choosing "Save as" in the _File_ menu and choosing the file format
"CSV UTF-8 (Comma-delimited) (.csv)". Save the file and, if you are using the
web app, upload it to your project.

In our case, we will be building a table about Moore's Law. For this purpose, we
are using a statistic with [how many transistors the average microprocessor
consists of per year from Our World in
Data](https://ourworldindata.org/grapher/transistors-per-microprocessor). Let's
start by pressing the "Download" button to get a CSV file with the raw data.

Be sure to move the file to your project or somewhere Typst can see it if you
are using the CLI. Once you did that, we can open the file to see how it is
structured:

```csv
Entity,Code,Year,Transistors per microprocessor
World,OWID_WRL,1971,2308.2417
World,OWID_WRL,1972,3554.5222
World,OWID_WRL,1974,6097.5625
```

The file starts with a header and contains four columns: Entity (which is to
whom the metric applies), Code, the year, and transistors per microprocessor.
Only the last two columns change between each row, so we can disregard "Entity"
and "Code".

First, let's start by loading this file with the [`csv`]($csv) function. It
accepts the file name of the file we want to load as a string argument:

```typ
#let moore = csv("moore.csv")
```

We have loaded our file (assuming we named it `moore.csv`) and [bound
it]($scripting/#bindings) to the new variable `moore`. This will not produce any
output, so there's nothing to see yet. If we want to examine what Typst loaded,
we can either hover the name of the variable in the web app or print some items
from the array:

```example
#let moore = csv("moore.csv")

#moore.slice(0, 3)
```

The [`slice`]($array.slice) method return the first three items in the array
(with the indices 0, 1, and 2) with these arguments. We can see that each row is
its own array with one item per cell.

Now, let's write a loop that will output an array of content that we can use
with the table function.

```example
#let moore = csv("moore.csv")

#table(
  columns: 2,
  ..for year in moore {
    (year.at(2), year.at(3))
  }
)
```

The example above uses a loop that iterates over the rows in our CSV file and
returns an array for each iteration. We could just write `year` in the body of
the loop but then we would also get the first two columns which we want to
discard. Instead, we create a new array containing the third and fourth column
only. Because Typst will concatenate the array results of the various loop
iterations, we get a one-dimensional array in which the year column and the
number of transistors alternate. Hence, we can insert the array as cells. For
this we use the [spread operator]($syntax) (`..`). By prefixing an array, or, in
our case an expression that yields an arry, with two dots, we tell Typst that
the array's items should be used as positional arguments.

We can also use the `map`, `slice` and `flatten` array methods to write this in
a more compact, functional style:

```typ
#let moore = csv("moore.csv")

#table(
   columns: moore.first().len(),
   ..moore.map(m => m.slice(2)).flatten(),
)
```

This example renders the same as the previous one, but first uses the
[`map`]($array.map) function to change each row of the data. We pass a function
to map that gets run on each row of the CSV and returns a new value to replace
that row with. We use it to discard the first two columns with `slice`. Then, we
spread the data into the `table` function. However, we need to pass a
one-dimensional array and `moore`'s value is two-dimensional (that means that
each of its row values contains an array with the cell data). That's why we call
`flatten` which converts it to a one-dimensional array. We also extract the
number of columns from the data itself.

Now that we have nice code for our table, we should try to also make the table
itself nice! The transistor counts go from millions in 1995 to trillions in 2021
and changes are difficult to see with so many digits. We could try to present
our data logarithmically to make it more digestible:

```example
#let moore = csv("moore.csv").slice(1).map(m => {
  let (.., year, count) = m
  let log = calc.log(float(count))
  let rounded = str(calc.round(log, digits: 2))
  (year, rounded)
})

#show table.cell.where(x: 0): strong

#table(
   columns: moore.first().len(),
   align: right,
   fill: (_, y) => if calc.odd(y) { rgb("D7D9E0") },
   stroke: none,

   table.header[Year][Transistor count ($log_10$)],
   table.hline(stroke: rgb("4D4C5B")),
   ..moore.flatten(),
)
```

In this example, we first drop the header row from the data since we are adding
our own. Then, we discard all but the last two columns as above. We do this by
[destructuring]($scripting/#bindings) the array `m`, discarding all but the two
last items. We then convert the string in `count` to a floating point number,
calculate its logarithm and store it in the variable `log`. Finally, we round it
to two digits, convert it to a string, and store it in the variable `rounded`.
Then, we return an array with `year` and `rounded` that replaces the original
row. In our table, we have added our custom header that tells the reader that
we've applied a logarithm to the values. Then, we spread the flattened data as
above.

We also styled the table with [stripes](#striped-rows-and-columns), a
[stroke](#strokes) below the first row, aligning everything right, and a bold
first column. Click on the links to go to the relevant guide sections and see
how its done!

## How to rotate a table? { #rotate-table }

When tables get a large number of columns, a portrait paper orientation can
quickly get cramped. Hence, you'll sometimes want to switch your tables to
landscape orientation. There are two ways to accomplish this in Typst:

- If you want to rotate only the table but not the other content of the page and
  the page itself, use the [`rotate` function]($rotate) with the `reflow`
  argument set to `{true}`.
- If you want to rotate the whole page the table is on, you can use the [`page`
  element]($page) with its `flipped` argument set to `{true}`. The header,
  footer, and page number will now also appear on the long edge of the page.
  This has the advantage that the table will appear right side up when read on a
  computer, but it also means that a page in your document has different
  dimensions than all the others, which can be jarring to your readers.

Below, we will demonstrate both techniques with a student gradebook table.

First, we rotate the table on the page. The example also places some text on the
right of the table.

```example
#set page("a5", numbering: "— 1 —")
>>> #set page(margin: auto)
#show: columns.with(2)

#show table.cell.where(y: 0): set text(weight: "bold")

#rotate(-90deg,
  reflow: true,

  table(
    columns: (1fr, ..(5 * (auto,))),
    inset: (x: .6em,),
    stroke: (_, y) => (
      x: 1pt,
      top: if y <= 1 { 1pt } else { 0pt },
      bottom: 1pt,
    ),
    align: (x, _) =>
      if x == 0 or x == 5 { left } else { right },

    table.header(
      [Student Name],
      [Assignment 1], [Assignment 2],
      [Mid-term], [Final Exam],
      [Total Grade],
    ),
    [Jane Smith], [78%], [82%], [75%], [80%], [B],
    [Alex Johnson], [90%], [95%], [94%], [96%], [A+],
    [John Doe], [85%], [90%], [88%], [92%], [A],
    [Maria Garcia], [88%], [84%], [89%], [85%], [B+],
    [Zhang Wei], [93%], [89%], [90%], [91%], [A-],
    [Marina Musterfrau], [96%], [91%], [74%], [69%], [B-],
  ),
)

#lorem(80)
```


What we have here is a two-column document on ISO A5 paper with page numbers on
the bottom. The table has six columns and contains a few customizations to
[stroke](#strokes), alignment and spacing. But the most important part is that
the table is wrapped in a call to the `rotate` function with the `reflow`
argument being `true`. This will make the table rotate 90 degrees
counterclockwise. The reflow argument is needed so that the table's rotation
affects the layout. If it was omitted, Typst would layout the page as if the
table was not rotated.

The example also shows how to produce many columns of the same size: After the
initial `{1fr}` column, we spread an array with five `{auto}` items that we
create by multiplying an array with one `{auto}` item by five.

The second example shows how to rotate the whole page so the table stays
upright:

```example
#set page("a5", numbering: "— 1 —")
>>> #set page(margin: auto)
#show table.cell.where(y: 0): set text(weight: "bold")

#page(flipped: true)[
  #table(
    columns: (1fr, ..(5 * (auto,))),
    inset: (x: .6em,),
    stroke: (_, y) => (
      x: 1pt,
      top: if y <= 1 { 1pt } else { 0pt },
      bottom: 1pt,
    ),
    align: (x, _) =>
      if x == 0 or x == 5 { left } else { right },

    table.header(
      [Student Name],
      [Assignment 1], [Assignment 2],
      [Mid-term], [Final Exam],
      [Total Grade],
    ),
    [Jane Smith], [78%], [82%], [75%], [80%], [B],
    [Alex Johnson], [90%], [95%], [94%], [96%], [A+],
    [John Doe], [85%], [90%], [88%], [92%], [A],
    [Maria Garcia], [88%], [84%], [89%], [85%], [B+],
    [Zhang Wei], [93%], [89%], [90%], [91%], [A-],
    [Marina Musterfrau], [96%], [91%], [74%], [69%], [B-],
  )

  #pad(x: 15%, top: 1.5em)[
    = Winter 2023/24 results
    #lorem(80)
  ]
]
```

Here, we take the same table and the other content we want to set with it and
put it in a call to the `page` function with while supplying `{true}` as the
`flipped` argument. This will instruct Typst create new pages with width and
height swapped and place the contents of the function call on the new page.
Notice how the page number is also on the long edge of the paper now. At the
bottom of the page, we use the [`pad`]($pad) function to constrain the width of
the paragraph to achieve a nice and legible line length.

## How to merge cells? { #merge-cells }

When a table contains logical groupings or the same data in multiple adjacent
cells, merging multiple cells in a single, larger cell can be advantageous.
Another use case for cell groups are table headers with multiple rows: That way,
you can group for example a sales data table by quarter in the first row and by
months in the second row.

A merged cell spans multiple rows and / or columns. You can achieve it with the
[`table.cell`]($table.cell) function's `rowspan` and `colspan` arguments: Just
specify how many rows or columns you want your cell to span.

The example below contains a attendance calendar for an office with in-person
and remote days for each team member. To make the table more glanceable, we
merge adjacent cells with the same value:

```example
>>> #set page(width: 22cm)
#let ofi = [Office]
#let rem = [_Remote_]
#let lea = [*On leave*]

#show table.cell.where(y: 0): set text(
  fill: white, weight: "bold"
)

#table(
  columns: 6 * (1fr,),
  align: (x, y) =>
    if y == 0 { bottom } else { horizon } +
    if x == 0 or y == 0 { left } else { center },
  stroke: (x, y) =>
    if y == 0 and x > 0 {
      (left: white, rest: black)
    } else { 1pt },
  fill: (_, y) => if y == 0 { black },

  table.header(
    [Team member],
    [Monday],
    [Tuesday],
    [Wednesday],
    [Thursday],
    [Friday]
  ),
  [Evelyn Archer],
    table.cell(colspan: 2, ofi),
    table.cell(colspan: 2, rem),
    ofi,
  [Lila Montgomery],
    table.cell(colspan: 5, lea),
  [Nolan Pearce],
    rem,
    table.cell(colspan: 2, ofi),
    rem,
    ofi,
)
```

In the example, we first define variables with "Office", "Remote", and "On
leave" so we don't have to write these labels out every time. We can then use
these variables in the table body either directly or in a `table.cell` call if
the team member spends multiple consecutive days in office, remote, or off.

The example also contains a black header (created with `table`'s `fill`
argument) with white strokes (`table`'s `stroke` argument) and white text (set
by the `table.cell` set rule). Finally, we align all the content of all table
cells in the body in their vertical (_`{horizon}`_) and horizontal
(_`{center}`_) center. If you want to know more about the functions passed to
`align`, `stroke`, and `fill`, you can check out the sections on
[alignment](#alignment), [strokes](#stroke-functions), and [striped
tables](#striped-rows-and-columns).

## How to get a striped table? { #striped-rows-and-columns }

### Changing the table cell's fill based on contents { #custom-cell-fill }

## How do I caption and reference my table? { #captions-and-references }

## How to break a table across pages? { #pagebreaks }

## How to align the contents of the cells in my table? { #alignment }

## What if I need the table function for something that isn't a table? { #grid }
