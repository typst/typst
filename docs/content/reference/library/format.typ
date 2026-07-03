#import "../../../components/index.typ": docs-category, scope

#let scope = scope(std, "format");
#show: docs-category.with(
  title: "Formats",
  description: "Documentation for Typst's export formats.",
  category: "format",
  scope: scope,
  sub-categories: scope.dict,
)

Some of the features in Typst only apply to certain output file formats.
Here you can find available format-specific settings and learn what features are available to customize your document for a given format.

= Setting default export options <setting-default-export-options>
Typst allows setting the default export options for a document directly from within it. These defaults can be overridden by CLI arguments or using the web-app `Export & Preview` panel. This is done using set rules on the format elements defined below.

Here is an example on how you can change the default @pdf.standard of a document.
```typ
#set format.pdf(standard: "ua-1")
```
