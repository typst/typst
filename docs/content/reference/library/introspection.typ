#import "../../../components/index.typ": docs-category

#show: docs-category.with(
  title: "Introspection",
  description: "Documentation for functionality that enables interactions between different parts of a document.",
  category: "introspection",
)

Interactions between document parts.

This category is home to Typst's introspection capabilities: With the `counter` function, you can access and manipulate page, section, figure, and equation counters or create custom ones. Meanwhile, the `query` function lets you search for elements in the document to construct things like a list of figures or headers which show the current chapter title.

Most of the functions are _contextual._ It is recommended to read the chapter on @reference:context[context] before continuing here.
