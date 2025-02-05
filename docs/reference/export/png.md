Instead of creating a PDF, Typst can also directly render pages to PNG raster
graphics. PNGs are losslessly compressed images that can contain one page at a
time. When exporting a multi-page document, Typst will emit multiple PNGs. PNGs
are a good choice when you want to use Typst's output in an image editing
software or when you can use none of Typst's other export formats.

In contrast to Typst's other export formats, PNGs are bound to a specific
resolution. When exporting to PNG, you can configure the resolution as pixels
per inch (PPI). If the medium you view the PNG on has a finer resolution than
the PNG you exported, you will notice a loss of quality. Typst calculates the
resolution of your PNGs based on each page's physical dimensions and the PPI. If
you need guidance for choosing a PPI value, consider the following:

- A DPI value of 300 or 600 is typical for desktop printing.
- Professional prints of detailed graphics can go up to 1200 PPI.
- If your document is only viewed at a distance, e.g. a poster, you may choose a
  smaller value than 300.
- If your document is viewed on screens, a typical PPI value for a smartphone is
  400-500.

Because PNGs only contain a pixel raster, the text within cannot be extracted
automatically (without OCR), for example by copy/paste or a screen reader. If
you need the text to be accessible, export a PDF or HTML file instead.

PNGs can have transparent backgrounds. By default, Typst will output a PNG with
an opaque white background. You can make the background transparent using
`[#set page(fill: none)]`. Learn more on the
[`page` function's reference page]($page.fill).

# Exporting as PNG
## Command Line
Pass `--format png` to the `compile` or `watch` subcommand or provide an output
file name that ends with `.png`.

If your document has more than one page, Typst will create multiple image files.
The output file name must then be a template string containing at least one of
- `[{p}]`, which will be replaced by the page number
- `[{0p}]`, which will be replaced by the zero-padded page number (so that all
  numbers have the same length)
- `[{t}]`, which will be replaced by the total number of pages

When exporting to PNG, you have the following configuration options:

- Which resolution to render at by specifying `--ppi` followed by a number of
  pixels per inch. The default is `144`.

- Which pages to export by specifying `--pages` followed by a comma-separated
  list of numbers or dash-separated number ranges. Ranges can be half-open.
  Example: `2,3,7-9,11-`.

## Web App
Click "File" > "Export as" > "PNG" or click the downwards-facing arrow next to
the quick download button and select "Export as PNG". When exporting to PNG, you
have the following configuration options:

- The resolution at which the pages should be rendered, as a number of pixels
  per inch. The default is `144`.

- Which pages to export. Valid options are "All pages", "Current page", and
  "Custom ranges". Custom ranges are a comma-separated list of numbers or
  dash-separated number ranges. Ranges can be half-open. Example: `2,3,7-9,11-`.
