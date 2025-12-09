Interactions between document parts.

This category is home to Typst's introspection capabilities: With the `counter`
function, you can access and manipulate page, section, figure, and equation
counters or create custom ones. Meanwhile, the `query` function lets you search
for elements in the document to construct things like a list of figures or
headers which show the current chapter title.

Most of the functions are _contextual._ It is recommended to read the chapter on
[context] before continuing here.


## Countable Elements

The `counter()` function can be used in three ways:

### 1. Built-in Element Counters
For counting built-in document elements that implement the `Locatable` trait:
- `page` - Page numbers
- `heading` - Headings (all levels)
- `figure` - Figures
- `equation` - Equations
- `footnote` - Footnotes
- `table` - Tables
- `list` - List items
- `enum` - Enumerated items

Example:

**typst
#let page-counter = counter(page)

### 2. Custom String Counters
For creating your own counters with string keys:

**typst
#let questions = counter("questions")
questions.step()
questions.display()  // Displays: 1

### 3. Label-based Counters
For counting specific labeled elements:
**typst
#let label = <appendix>
#let appendix-counter = counter(label)
#heading(label)[Appendix]
#appendix-counter.display()

*Note: This list of built-in elements is based on Typst v0.10.0. The available elements may change in future versions.*
