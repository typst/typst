#import "../../../components/index.typ": docs-category, short-or-long, scope

#show: docs-category.with(
  title: "Formats",
  description: "Documentation for Typst's export formats.",
  category: "format",
  scope: scope(std, "format"),
  sub-categories: dictionary(format),
)

Typst documents can be exported to various different export formats. This section documents the available export formats and lists format-specific settings and features.

Each export format has an associated element through which it can be configured. These elements are defined in the global `format` module. The major formats `pdf` and `html` are additionally directly available in the global scope.

= #short-or-long[Export setstings][Configuring export settings] <export-settings>
There are two ways to specify export settings:

- At export time, through command line arguments or the web app's "Export & Preview"  panel
- In the document, through a @reference:styling:set-rules[set rule] on the format's element

Below is an example showing how you could change the default @pdf.standard[PDF standard] for a document. Unless specified otherwise (e.g. via a command line argument), a document with this set rule will be exported as a PDF/UA-1.

```typ
// The document will now default to PDF/UA-1.
#set pdf(standard: "ua-1")
```

Similarly, we can write a set rule that makes rendered PNG a bit sharper by default. Here, we have to write @format.png instead of just `png` since the PNG format element is not globally available (as it's a bit more niche of an export format than PDF).

```typ
// This makes PNGs a bit higher-res.
#set format.png(ppi: 300)
```
