Instead of creating a PDF, Typst can also directly render pages to scalable
vector graphics (SVGs), which are the preferred format for embedding vector
graphics in web pages. Like PDF files, SVGs display your document exactly how
you have laid it out in Typst. Likewise, they share the benefit of not being
bound to a specific resolution. Hence, you can print or view SVG files on any
device without incurring a loss of quality. (Note that font printing quality may
be better with a PDF.) In contrast to a PDF, an SVG cannot contain multiple
pages. When exporting a multi-page document, Typst will emit multiple SVGs.

SVGs can represent text in two ways: By embedding the text itself and rendering
it with the fonts available on the viewer's computer or by embedding the shapes
of each glyph in the font used to create the document. To ensure that the SVG
file looks the same across all devices it is viewed on, Typst chooses the latter
method. This means that the text in the SVG cannot be extracted automatically,
for example by copy/paste or a screen reader. If you need the text to be
accessible, export a PDF or HTML file instead.

SVGs can have transparent backgrounds. By default, Typst will output an SVG with
an opaque white background. You can make the background transparent using
`[#set page(fill: none)]`. Learn more on the
[`page` function's reference page]($page.fill).

# Exporting as SVG
## Command Line
Pass `--format svg` to the `compile` or `watch` subcommand or provide an output
file name that ends with `.svg`.

If your document has more than one page, Typst will create multiple image files.
The output file name must then be a template string containing at least one of
- `[{p}]`, which will be replaced by the page number
- `[{0p}]`, which will be replaced by the zero-padded page number (so that all
  numbers have the same length)
- `[{t}]`, which will be replaced by the total number of pages

When exporting to SVG, you have the following configuration options:

- Which pages to export by specifying `--pages` followed by a comma-separated
  list of numbers or dash-separated number ranges. Ranges can be half-open.
  Example: `2,3,7-9,11-`.

## Web App
Click "File" > "Export as" > "SVG" or click the downwards-facing arrow next to
the quick download button and select "Export as SVG". When exporting to SVG, you
have the following configuration options:

- Which pages to export. Valid options are "All pages", "Current page", and
  "Custom ranges". Custom ranges are a comma-separated list of numbers or
  dash-separated number ranges. Ranges can be half-open. Example: `2,3,7-9,11-`.
