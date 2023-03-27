# None
A value that indicates the absence of any other value.

The none type has exactly one value: `{none}`.

When inserted into the document, it is not visible.
This is also the value that is produced by empty code blocks.
It can be [joined]($scripting/#blocks) with any value, yielding
the other value.

## Example
```example
Not visible: #none
```

# Auto
A value that indicates a smart default.

The auto type has exactly one value: `{auto}`.

Parameters that support the `{auto}` value have some smart default or contextual
behaviour. A good example is the [text direction]($func/text.dir) parameter.
Setting it to `{auto}` lets Typst automatically determine the direction from the
[text language]($func/text.lang).

# Boolean
Either `{true}` or `{false}`.

The boolean type has two values: `{true}` and `{false}`. It denotes whether
something is active or enabled.

## Example
```example
#false \
#true \
#(1 < 2)
```

# Integer
A whole number.

The number can be negative, zero, or positive. As Typst uses 64 bits to store
integers, integers cannot be smaller than `{-9223372036854775808}` or larger than
`{9223372036854775807}`.

## Example
```example
#(1 + 2) \
#(2 - 5) \
#(3 + 4 < 8)
```

# Float
A floating-pointer number.

A limited-precision representation of a real number. Typst uses 64 bits to
store floats. Wherever a float is expected, you can also pass an
[integer]($type/integer).

## Example
```example
#3.14 \
#1e4 \
#(10 / 4)
```

# Length
A size or distance, possibly expressed with contextual units.
Typst supports the following length units:

- Points: `{72pt}`
- Millimeters: `{254mm}`
- Centimeters: `{2.54cm}`
- Inches: `{1in}`
- Relative to font size: `{2.5em}`

## Example
```example
#rect(width: 20pt)
#rect(width: 2em)
#rect(width: 1in)
```

# Angle
An angle describing a rotation.
Typst supports the following angular units:

- Degrees: `{180deg}`
- Radians: `{3.14rad}`

## Example
```example
#rotate(10deg)[Hello there!]
```

# Ratio
A ratio of a whole.

Written as a number, followed by a percent sign.

## Example
```example
#set align(center)
#scale(x: 150%)[
  Scaled apart.
]
```

# Relative Length
A length in relation to some known length.

This type is a combination of a [length]($type/length) with a
[ratio]($type/ratio). It results from addition and subtraction
of a length and a ratio. Wherever a relative length is expected, you can also
use a bare length or ratio.

## Example
```example
#rect(width: 100% - 50pt)
```

# Fraction
Defines how the the remaining space in a layout is distributed.

Each fractionally sized element gets space based on the ratio of its fraction to
the sum of all fractions.

For more details, also see the [h]($func/h) and [v]($func/v) functions and the
[grid function]($func/grid).

## Example
```example
Left #h(1fr) Left-ish #h(2fr) Right
```

# Color
A color in a specific color space.

Typst supports:
- sRGB through the [`rgb` function]($func/rgb)
- Device CMYK through [`cmyk` function]($func/cmyk)
- D65 Gray through the [`luma` function]($func/luma)

Furthermore, Typst provides the following built-in colors:

`black`, `gray`, `silver`, `white`, `navy`, `blue`, `aqua`, `teal`, `eastern`,
`purple`, `fuchsia`, `maroon`, `red`, `orange`, `yellow`, `olive`, `green`, and
`lime`.

## Methods
### lighten()
Lightens a color.

- amount: ratio (positional, required)
  The factor to lighten the color by.
- returns: color

### darken()
Darkens a color.

- amount: ratio (positional, required)
  The factor to darken the color by.
- returns: color

### negate()
Produces the negative of the color.

- returns: color

# Symbol
A Unicode symbol.

Typst defines common symbols so that they can easily be written with standard
keyboards. The symbols are defined in modules, from which they can be accessed
using [field access notation]($scripting/#fields):

- General symbols are defined in the [`sym` module]($category/symbols/sym)
- Emoji are defined in the [`emoji` module]($category/symbols/emoji)

Moreover, you can define custom symbols with the [symbol]($func/symbol)
function.

```example
#sym.arrow.r \
#sym.gt.eq.not \
$gt.eq.not$ \
#emoji.face.halo
```

Many symbols have different variants, which can be selected by appending the
modifiers with dot notation. The order of the modifiers is not relevant. Visit
the documentation pages of the symbol modules and click on a symbol to see its
available variants.

```example
$arrow.l$ \
$arrow.r$ \
$arrow.t.quad$
```

# String
A sequence of Unicode codepoints.

You can iterate over the grapheme clusters of the string using a
[for loop]($scripting/#loops). Grapheme clusters are basically characters but
keep together things that belong together, e.g. multiple codepoints that
together form a flag emoji. Strings can be added with the `+` operator,
[joined together]($scripting/#blocks) and multiplied with integers.

Typst provides utility methods for string manipulation. Many of these methods
(e.g., `split`, `trim` and `replace`) operate on _patterns:_ A pattern can be
either a string or a [regular expression]($func/regex). This makes the methods
quite versatile.

All lengths and indices are expressed in terms of UTF-8 bytes.

### Example
```example
#"hello world!" \
#"\"hello\n  world\"!" \
#"1 2 3".split() \
#"1,2;3".split(regex("[,;]")) \
#(regex("\d+") in "ten euros") \
#(regex("\d+") in "10 euros")
```

### Escape sequences
Just like in markup, you can escape a few symbols in strings:
- `[\\]` for a backslash
- `[\"]` for a quote
- `[\n]` for a newline
- `[\r]` for a carriage return
- `[\t]` for a tab
- `[\u{1f600}]` for a hexadecimal Unicode escape sequence

## Methods
### len()
The length of the string in UTF-8 encoded bytes.

- returns: integer

### first()
Extract the first grapheme cluster of the string.
Fails with an error if the string is empty.

- returns: any

### last()
Extract the last grapheme cluster of the string.
Fails with an error if the string is empty.

- returns: any

### at()
Extract the first grapheme cluster after the specified index. Fails with an
error if the index is out of bounds.

- index: integer (positional, required)
  The byte index.
- returns: string

### slice()
Extract a substring of the string.
Fails with an error if the start or end index is out of bounds.

- start: integer (positional, required)
  The start byte index (inclusive).
- end: integer (positional)
  The end byte index (exclusive). If omitted, the whole slice until the end of the
  string is extracted.
- count: integer (named)
  The number of bytes to extract. This is equivalent to passing `start + count`
  as the `end` position. Mutually exclusive with `end`.
- returns: string

### clusters()
Returns the grapheme clusters of the string as an array of substrings.

- returns: array

### codepoints()
Returns the Unicode codepoints of the string as an array of substrings.

- returns: array

### contains()
Whether the string contains the specified pattern.

This method also has dedicated syntax: You can write `{"bc" in "abcd"}` instead
of `{"abcd".contains("bc")}`.

- pattern: string or regex (positional, required)
  The pattern to search for.
- returns: boolean

### starts-with()
Whether the string starts with the specified pattern.

- pattern: string or regex (positional, required)
  The pattern the string might start with.
- returns: boolean

### ends-with()
Whether the string ends with the specified pattern.

- pattern: string or regex (positional, required)
  The pattern the string might end with.
- returns: boolean

### find()
Searches for the specified pattern in the string and returns the first match
as a string or `{none}` if there is no match.

- pattern: string or regex (positional, required)
  The pattern to search for.
- returns: string or none

### position()
Searches for the specified pattern in the string and returns the index of the
first match as an integer or `{none}` if there is no match.

- pattern: string or regex (positional, required)
  The pattern to search for.
- returns: integer or none

### match()
Searches for the specified pattern in the string and returns a dictionary
with details about the first match or `{none}` if there is no match.

The returned dictionary has the following keys:
* `start`: The start offset of the match
* `end`: The end offset of the match
* `text`: The text that matched.
* `captures`: An array containing a string for each matched capturing group. The
  first item of the array contains the first matched capturing, not the whole
  match! This is empty unless the `pattern` was a regex with capturing groups.

- pattern: string or regex (positional, required)
  The pattern to search for.
- returns: dictionary or none

### matches()
Searches for the specified pattern in the string and returns an array of
dictionaries with details about all matches. For details about the returned
dictionaries, see above.

- pattern: string or regex (positional, required)
  The pattern to search for.
- returns: array

### replace()
Replaces all or a specified number of matches of a pattern with a replacement
string and returns the resulting string.

- pattern: string or regex (positional, required)
  The pattern to search for.
- replacement: string (positional, required)
  The string to replace the matches with.
- count: integer (named)
  If given, only the first `count` matches of the pattern are placed.
- returns: string

### trim()
Removes matches of a pattern from one or both sides of the string, once or
repeatedly and returns the resulting string.

- pattern: string or regex (positional, required)
  The pattern to search for.
- at: alignment (named)
  Can be `start` or `end` to only trim the start or end of the string.
  If omitted, both sides are trimmed.
- repeat: boolean (named)
  Whether to repeatedly removes matches of the pattern or just once.
  Defaults to `{true}`.
- returns: string

### split()
Splits a string at matches of a specified pattern and returns an array of
the resulting parts.

- pattern: string or regex (positional)
  The pattern to split at. Defaults to whitespace.
- returns: array

# Content
A piece of document content.

This type is at the heart of Typst. All markup you write and most
[functions]($type/function) you call produce content values. You can create a
content value by enclosing markup in square brackets. This is also how you pass
content to functions.

```example
Type of *Hello!* is
#type([*Hello!*])
```

Content can be added with the `+` operator,
[joined together]($scripting/#blocks) and multiplied with
integers. Wherever content is expected, you can also pass a
[string]($type/string) or `{none}`.

## Representation
Content consists of elements with fields. When constructing an element with
its _element function,_ you provide these fields as arguments and when you have
a content value, you can access its fields with
[field access syntax]($scripting/#field-access).

Some fields are required: These must be provided when constructing an element
and as a consequence, they are always available through field access on content
of that type. Required fields are marked as such in the documentation.

Most fields are optional: Like required fields, they can be passed to the
element function to configure them for a single element. However, these can also
be configured with [set rules]($styling/#set-rules) to apply them to all
elements within a scope. Optional fields are only available with field access
syntax when they are were explicitly passed to the element function, not when
they result from a set rule.

Each element has a default appearance. However, you can also completely
customize its appearance with a [show rule]($styling/#show-rules). The show rule
is passed the element. It can access the element's field and produce arbitrary
content from it.

In the web app, you can hover over a content variable to see exactly which
elements the content is composed of and what fields they have. Alternatively,
you can inspect the output of the [`repr`]($func/repr) function.

## Methods
### func()
The content's element function. This function can be used to create the element
contained in this content. It can be used in set and show rules for the element.
Can be compared with global functions to check whether you have a specific
kind of element.

- returns: function

### has()
Whether the content has the specified field.

- field: string (positional, required)
  The field to look for.
- returns: boolean

### at()
Access the specified field on the content.

- field: string (positional, required)
  The field to access.
- returns: any

### location()
The location of the content. This is only available on content returned by
[query]($func/query), for other content it will fail with an error. The
resulting location can be used with [counters]($func/counter),
[state]($func/state) and [queries]($func/query).

- returns: location

# Array
A sequence of values.

You can construct an array by enclosing a comma-separated sequence of values
in parentheses. The values do not have to be of the same type.

You can access and update array items with the `.at()` method. Indices are
zero-based and negative indices wrap around to the end of the array. You can
iterate over an array using a [for loop]($scripting/#loops).
Arrays can be added together with the `+` operator,
[joined together]($scripting/#blocks) and multiplied with
integers.

**Note:** An array of length one needs a trailing comma, as in `{(1,)}`. This is
to disambiguate from a simple parenthesized expressions like `{(1 + 2) * 3}`.
An empty array is written as `{()}`.

## Example
```example
#let values = (1, 7, 4, -3, 2)

#values.at(0) \
#(values.at(0) = 3)
#values.at(-1) \
#values.find(calc.even) \
#values.filter(calc.odd) \
#values.map(calc.abs) \
#values.rev() \
#(1, (2, 3)).flatten() \
#(("A", "B", "C")
    .join(", ", last: " and "))
```

## Methods
### len()
The number of values in the array.

- returns: integer

### first()
Returns the first item in the array.
May be used on the left-hand side of an assignment.
Fails with an error if the array is empty.

- returns: any

### last()
Returns the last item in the array.
May be used on the left-hand side of an assignment.
Fails with an error if the array is empty.

- returns: any

### at()
Returns the item at the specified index in the array.
May be used on the left-hand side of an assignment.
Fails with an error if the index is out of bounds.

- index: integer (positional, required)
  The index at which to retrieve the item.
- returns: any

### push()
Add a value to the end of the array.

- value: any (positional, required)
  The value to insert at the end of the array.

### pop()
Remove the last item from the array and return it.
Fails with an error if the array is empty.

- returns: any
  The removed last value.

### insert()
Insert a value into the array at the specified index.
Fails with an error if the index is out of bounds.

- index: integer (positional, required)
  The index at which to insert the item.
- value: any (positional, required)
  The value to insert into the array.

### remove()
Remove the value at the specified index from the array and return it.

- index: integer (positional, required)
  The index at which to remove the item.
- returns: any

### slice()
Extract a subslice of the array.
Fails with an error if the start or index is out of bounds.

- start: integer (positional, required)
  The start index (inclusive).
- end: integer (positional)
  The end index (exclusive). If omitted, the whole slice until the end of the
  array is extracted.
- count: integer (named)
  The number of items to extract. This is equivalent to passing `start +
  count` as the `end` position. Mutually exclusive with `end`.
- returns: array

### contains()
Whether the array contains the specified value.

This method also has dedicated syntax: You can write `{2 in (1, 2, 3)}` instead
of `{(1, 2, 3).contains(2)}`.

- value: any (positional, required)
  The value to search for.
- returns: boolean

### find()
Searches for an item for which the given function returns `{true}` and
returns the first match or `{none}` if there is no match.

- searcher: function (positional, required)
  The function to apply to each item. Must return a boolean.
- returns: any or none

### position()
Searches for an item for which the given function returns `{true}` and
returns the index of the first match or `{none}` if there is no match.

- searcher: function (positional, required)
  The function to apply to each item. Must return a boolean.
- returns: integer or none

### filter()
Produces a new array with only the items from the original one for which the
given function returns true.

- test: function (positional, required)
  The function to apply to each item. Must return a boolean.
- returns: array

### map()
Produces a new array in which all items from the original one were
transformed with the given function.

- mapper: function (positional, required)
  The function to apply to each item.
- returns: array

### fold()
Folds all items into a single value using an accumulator function.

- init: any (positional, required)
  The initial value to start with.
- folder: function (positional, required)
  The folding function. Must have two parameters: One for the accumulated value
  and one for an item.
- returns: any

### any()
Whether the given function returns `{true}` for any item in the array.

- test: function (positional, required)
  The function to apply to each item. Must return a boolean.
- returns: boolean

### all()
Whether the given function returns `{true}` for all items in the array.

- test: function (positional, required)
  The function to apply to each item. Must return a boolean.
- returns: boolean

### flatten()
Combine all nested arrays into a single flat one.

- returns: array

### rev()
Return a new array with the same items, but in reverse order.

- returns: array

### join()
Combine all items in the array into one.

- separator: any (positional)
  A value to insert between each item of the array.
- last: any (named)
  An alternative separator between the last two items
- returns: any

### sorted()
Return a new array with the same items, but sorted.

- returns: array

# Dictionary
A map from string keys to values.

You can construct a dictionary by enclosing comma-separated `key: value` pairs
in parentheses. The values do not have to be of the same type.

A dictionary is conceptually similar to an array, but it is indexed by strings
instead of integers. You can access and create dictionary entries with the
`.at()` method. If you know the key statically, you can alternatively use
[field access notation]($scripting/#fields) (`.key`) to access
the value. Dictionaries can be added with the `+` operator and
[joined together]($scripting/#blocks).
To check whether a key is present in the dictionary, use the `in` keyword.

You can iterate over the pairs in a dictionary using a
[for loop]($scripting/#loops).
Dictionaries are always ordered by key.

Since empty parentheses already yield an empty array, you have to use the
special `(:)` syntax to create an empty dictionary.


## Example
```example
#let dict = (
  name: "Typst",
  born: 2019,
)

#dict.name \
#(dict.launch = 20)
#dict.len() \
#dict.keys() \
#dict.values() \
#dict.at("born") \
#dict.insert("city", "Berlin ")
#("name" in dict)
```

## Methods
### len()
The number of pairs in the dictionary.

- returns: integer

### at()
Returns the value associated with the specified key in the dictionary.
May be used on the left-hand side of an assignment if the key is already
present in the dictionary.
Fails with an error if the key is not part of the dictionary.

- key: string (positional, required)
  The key at which to retrieve the item.
- returns: any

### insert()
Insert a new pair into the dictionary and return the value.
If the dictionary already contains this key, the value is updated.

- key: string (positional, required)
  The key of the pair that should be inserted.
- value: any (positional, required)
  The value of the pair that should be inserted.

### keys()
Returns the keys of the dictionary as an array in sorted order.

- returns: array

### values()
Returns the values of the dictionary as an array in key-order.

- returns: array

### pairs()
Returns the keys and values of the dictionary as an array of pairs. Each pair is
represented as an array of length two.

- returns: array

### remove()
Remove a pair from the dictionary by key and return the value.

- key: string (positional, required)
  The key of the pair that should be removed.
- returns: any

# Function
A mapping from argument values to a return value.

You can call a function by writing a comma-separated list of function
_arguments_ enclosed in parentheses directly after the function name.
Additionally, you can pass any number of trailing content blocks arguments to a
function _after_ the normal argument list. If the normal argument list would
become empty, it can be omitted. Typst supports positional and named arguments.
The former are identified by position and type, while the later are written as
`name: value`.

Within math mode, function calls have special behaviour. See the
[math documentation]($category/math) for more details.

### Example
```example
// Call a function.
#list([A], [B])

// Named arguments and trailing
// content blocks.
#enum(start: 2)[A][B]

// Version without parentheses.
#list[A][B]
```

Functions are a fundamental building block of Typst. Typst provides functions
for a variety of typesetting tasks. Moreover, the markup you write is backed by
functions and all styling happens through functions. This reference lists all
available functions and how you can use them. Please also refer to the
documentation about [set]($styling/#set-rules) and
[show]($styling/#show-rules) rules to learn about additional ways
you can work with functions in Typst.

### Defining functions { #definitions }
You can define your own function with a
[let binding]($scripting/#bindings) that has a parameter list after
the binding's name. The parameter list can contain positional parameters,
named parameters with default values and
[argument sinks]($type/arguments).
The right-hand side of the binding can be a block or any other expression. It
defines the function's return value and can depend on the parameters.

```example
#let alert(body, fill: red) = {
  set text(white)
  set align(center)
  rect(
    fill: fill,
    inset: 8pt,
    radius: 4pt,
    [*Warning:\ #body*],
  )
}

#alert[
  Danger is imminent!
]

#alert(fill: blue)[
  KEEP OFF TRACKS
]
```

### Unnamed functions { #unnamed }
You can also created an unnamed function without creating a binding by
specifying a parameter list followed by `=>` and the function body. If your
function has just one parameter, the parentheses around the parameter list are
optional. Unnamed functions are mainly useful for show rules, but also for
settable properties that take functions like the page function's
[`footer`]($func/page.footer) property.

```example
#show "once?": it => [#it #it]
once?
```

### Notable fact
In Typst, all functions are _pure._ This means that for the same
arguments, they always return the same result. They cannot "remember" things to
produce another value when they are called a second time.

## Methods
### with()
Returns a new function that has the given arguments pre-applied.

- arguments: any (variadic)
  The named and positional arguments to apply.
- returns: function

### where()
Returns a selector that filters for elements belonging to this function
whose fields have the values of the given arguments.

- fields: any (named, variadic)
  The field values to filter by.
- returns: selector

# Arguments
Captured arguments to a function.

Like built-in functions, custom functions can also take a variable number of
arguments. You can specify an _argument sink_ which collects all excess
arguments as `..sink`. The resulting `sink` value is of the `arguments` type. It
exposes methods to access the positional and named arguments and is iterable
with a [for loop]($scripting/#loops). Inversely, you can spread
arguments, arrays and dictionaries into a function call with the spread operator:
`{func(..args)}`.

## Example
```example
#let format(title, ..authors) = [
  *#title* \
  _Written by #(authors
    .pos()
    .join(", ", last: " and "));._
]

#format("ArtosFlow", "Jane", "Joe")
```

## Methods
### pos()
Returns the captured positional arguments as an array.

- returns: array

### named()
Returns the captured named arguments as a dictionary.

- returns: dictionary

# Module
An evaluated module, either built-in or resulting from a file.

You can access definitions from the module using
[field access notation]($scripting/#fields) and interact with it using the
[import and include syntaxes]($scripting/#modules).

## Example
```example
<<< #import "utils.typ"
<<< #utils.add(2, 5)

<<< #import utils: sub
<<< #sub(1, 4)
>>> #7
>>>
>>> #(-3)
```
