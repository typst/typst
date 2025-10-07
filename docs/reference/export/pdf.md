PDF files focus on accurately describing documents visually, but also have
facilities for annotating their structure. This hybrid approach makes
them a good fit for document exchange: They render exactly the same on every
device, but also support extraction of a document's content and structure (at
least to an extent). Unlike PNG files, PDFs are not bound to a specific
resolution. Hence, you can view them at any size without incurring a loss of
quality.

# Exporting as PDF
## Command Line
PDF is Typst's default export format. Running the `compile` or `watch`
subcommand without specifying a format will create a PDF. When exporting to PDF,
you have the following configuration options:

- Which [PDF standards](#pdf-standards) Typst should enforce conformance with by
  specifying `--pdf-standard` followed by one or multiple comma-separated
  standards. Valid standards are `1.4`, `1.5`, `1.6`, `1.7`, `2.0`, `a-1b`,
  `a-1a`, `a-2b`, `a-2u`, `a-2a`, `a-3b`. `a-3u`, `a-3a`, `a-4`, `a-4f`, `a-4e`,
  and `ua-1`. By default, Typst outputs PDF-1.7-compliant files.

- You can disable PDF tagging completely with `--no-pdf-tags`. By default, Typst
  will always write _Tagged PDF_ to provide a baseline level of accessibility.
  Using this flag, you can turn tags off. This will make your file inaccessible
  and prevent conformance with accessible conformance levels of PDF/A and all
  parts of PDF/UA.

- Which pages to export by specifying `--pages` followed by a comma-separated
  list of numbers or dash-separated number ranges. Ranges can be half-open.
  Example: `2,3,7-9,11-`.

## Web App
Click the quick download button at the top right to export a PDF with default
settings. For further configuration, click "File" > "Export as" > "PDF" or click
the downwards-facing arrow next to the quick download button and select "Export
as PDF". When exporting to PDF, you have the following configuration options:

- Which PDF standards Typst should enforce conformance with. By default, Typst
  outputs PDF-1.7-compliant files. You can choose the PDF version freely between
  1.4 and 2.0. Valid additional standards are `A-1b`, `A-1a`, `A-2b`, `A-2u`,
  `A-2a`, `A-3b`. `A-3u`, `A-3a`, `A-4`, `A-4f`, `A-4e`, and `UA-1`.

- Which pages to export. Valid options are "All pages", "Current page", and
  "Custom ranges". Custom ranges are a comma-separated list of numbers or
  dash-separated number ranges. Ranges can be half-open. Example: `2,3,7-9,11-`.

# PDF standards
The International Standards Organization (ISO) has published the base PDF
standard and various standards that extend it to make PDFs more suitable for
specific use-cases. By default, Typst exports PDF 1.7 files. Adobe Acrobat 8 and
later as well as all other commonly used PDF viewers are compatible with this
PDF version. Some features of Typst may not be available depending on the PDF
standard you choose.

## PDF/UA
Typst supports writing PDF/UA-conformant files. PDF/UA files are designed
for _[Universal Access]($guides/accessibility/#basics)._ When you choose this PDF
standard, Typst will run additional checks when exporting your document. These
checks will make sure that you are following accessibility best practices. For
example, it will make sure that all your images come with alternative
descriptions.

Note that there are some rules in PDF/UA that are crucial for accessibility but
cannot be automatically checked. Hence, when exporting a PDF/UA-1 document, make
sure you did the following:

- If your document is written in a different language than English, make sure
  [set the text language]($text.lang) before any content.
- Make sure to use Typst's semantic elements (like [headings]($heading),
  [figures]($figure), and [lists]($list)) when appropriate instead of defining
  custom constructs. This lets Typst know (and export) the role a construct
  plays in the document. See [the Accessibility guide]($guides/accessibility)
  for more details.
- Do not exclusively use contrast, color, format, or layout to communicate an
  idea. Use text or alternative descriptions instead of or in addition to these
  elements.
- Wrap all decorative elements without a semantic meaning in [`pdf.artifact`].
- Do not use images of text. Instead, insert the text directly into your markup.

Typst currently only supports part one (PDF/UA-1) which is based on PDF 1.7
(2006). When exporting to PDF/UA-1, be aware that you will need to manually
provide [alternative descriptions of mathematics]($category/math/#accessibility)
in natural language.

New accessibility features were added to PDF 2.0 (2017). When set to PDF 2.0
export, Typst will leverage some of these features. PDF 2.0 and PDF/UA-1,
however, are mutually incompatible. For accessible documents, we currently
recommend exporting to PDF/UA-1 instead of PDF 2.0 for the additional checks and
greater compatibility. The second part of PDF/UA is designed for PDF 2.0, but
not yet supported by Typst.

## PDF/A
Typst optionally supports emitting PDF/A-conformant files. PDF/A files are
geared towards maximum compatibility with current and future PDF tooling. They
do not rely on difficult-to-implement or proprietary features and contain
exhaustive metadata. This makes them suitable for long-term archival.

The PDF/A Standard has multiple versions (_parts_ in ISO terminology) and most
parts have multiple profiles that indicate the file's conformance level. You can
target one part and conformance level at a time. Currently, Typst supports these
PDF/A output profiles:

- **PDF/A-1b:** The _basic_ conformance level of ISO 19005-1. This version of
  PDF/A is based on PDF 1.4 (2001) and results in self-contained, archivable PDF
  files. As opposed to later parts of the PDF/A standard, transparency is not
  allowed in PDF/A-1 files.

- **PDF/A-1a:** This is the _accessible_ conformance level that builds on the
  basic level PDF/A-1b. To conform to this level, your file must be an
  accessible _Tagged PDF_ file. Note that accessibility is improved with later
  parts as not all PDF accessibility features are available for PDF 1.4 files.
  Furthermore, all text in the file must consist of known Unicode code points.

- **PDF/A-2b:** The _basic_ conformance level of ISO 19005-2. This version of
  PDF/A is based on PDF 1.7 (2006) and results in self-contained, archivable PDF
  files.

- **PDF/A-2u:** This is the _Unicode-mappable_ conformance level that builds on
  the basic level A-2b. It also adds rules that all text in the document must
  consist of known Unicode code points. If possible, always prefer this standard
  over PDF/A-2b.

- **PDF/A-2a:** This is the _accessible_ conformance level that builds on the
  Unicode-mappable level A-2u. This conformance level also adds two
  requirements: Your file must be an accessible _Tagged PDF_ file. Typst
  automatically adds tags to help you reach this conformance level. Also pay
  attention to the Accessibility Sections throughout the reference and the
  [Accessibility Guide]($guides/accessibility) when targeting this conformance
  level. Finally, PDF/A2-a forbids you from using code points in the [Unicode
  Private Use area](https://en.wikipedia.org/wiki/Private_Use_Areas). If you
  want to build an accessible file, also consider additionally targeting
  PDF/UA-1, which enables more automatic accessibility checks.

- **PDF/A-3b:** The _basic_ conformance level of ISO 19005-3. This version of
  PDF/A is based on PDF 1.7 (2006) and results in archivable PDF files that can
  contain arbitrary other related files as [attachments]($pdf.attach). The only
  difference between it and PDF/A-2b is the capability to attach
  non-PDF/A-conformant files.

- **PDF/A-3u:** This is the _Unicode-mappable_ conformance level that builds on
  the basic level A-3b. Just like PDF/A-2b, this requires all text to consist
  of known Unicode code points. These rules do not apply to attachments. If
  possible, always prefer this standard over PDF/A-3b.

- **PDF/A-3a:** This is the _accessible_ conformance level that builds on the
  Unicode-mappable level A-3u. Just like PDF/A-2a, this requires files to be
  accessible _Tagged PDF_ and to not use characters from the Unicode Private Use
  area. Just like before, these rules do not apply to attachments.

- **PDF/A-4:** The basic conformance level of ISO 19005-4. This version of PDF/A
  is based on PDF 2.0 (2017) and results in self-contained, archivable PDF
  files. PDF/A-4 has no parts relating to accessibility. Instead, the topic has
  been elaborated on more in the dedicated PDF/UA standard. PDF/A-4 files can
  conform to PDF/UA-2 (currently not supported in Typst).

- **PDF/A-4f:** The _embedded files_ conformance level that builds on the basic
  level A-4. Files conforming to this level can contain arbitrary other related
  files as [attachments]($pdf.attach), just as files conforming to part 3 of ISO
  19005. The only difference between it and PDF/A-4 is the capability to attach
  non-PDF/A-conformant files.

- **PDF/A-4e:** The _engineering_ conformance level that builds on the embedded
  files level A-4f. Files conforming to this level can contain 3D objects. Typst
  does not support 3D content, so this is functionally equivalent to PDF/A4-f
  from a Typst perspective.

If you want to target PDF/A but are unsure about which particular setting to
use, there are some good rules of thumb. First, you must determine your
**part,** that's the "version number" of the standard. Ask yourself these
questions:

1. **Does pre-2006 software or equipment need to be able to read my file?** If
   so, choose part one (PDF/A-1).

2. **If not, does my file need [attachments]($pdf.attach)?** If so, you must
   choose part three (PDF/A-3) or the embedded files level of part four
   (PDF/A-4f).

3. **Does your file need to use features introduced in PDF 2.0?** Currently, use
   of PDF 2.0 features in Typst is limited to minor improvements in
   accessibility, e.g. for the [title element]($title). If you can do without
   these improvements, maximize compatibilty by choosing part two (when you
   don't need attachments) or part three (when you do need attachments). If you
   rely on PDF 2.0 features, use part four.

Now, you only need to choose a **conformance level** (the lowercase letter at
the end).

- **If you decided on part one, two, or three,** you should typically choose the
  accessible conformance level of your part, indicated by a lowercase `a` at the
  end. If your document is inherently inaccessible, e.g. an artist's portfolio
  that cannot be boiled down to alternative descriptions, choose conformance
  level `u` instead. Only if this results in a compiler error, e.g. because you
  used code points from the Unicode Private Use Area, use the basic level `b`.

- **If you have chosen part four,** you should choose the basic conformance
  level (PDF/A-4) except when needing to embed files.

When choosing between exporting PDF/A and regular PDF, keep in mind that PDF/A
files contain additional metadata, and that some readers will prevent the user
from modifying a PDF/A file.

# PDF-specific functionality
Typst exposes PDF-specific functionality in the global `pdf` module. See below
for the definitions it contains.

This module contains some functions without a final API. They are designed to
enhance accessibility for documents with complex tables. This includes
[`table-summary`]($pdf.table-summary), [`header-cell`]($pdf.header-cell), and
[`data-cell`]($pdf.data-cell). All of these functions will be removed in a
future Typst release, either through integration into table functions or through
full removal. You can enable these functions by passing `--features a11y-extras`
or setting the `TYPST_FEATURES` environment variable to `a11y-extras`. In the
web app, these features are not available at this time.
