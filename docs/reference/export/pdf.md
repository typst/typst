PDF files focus on accurately describing documents visually, but also have
facilities for annotating their structure. This hybrid approach makes
them a good fit for document exchange: They render exactly the same on every
device, but also support extraction of a document's content and structure (at
least to an extent). Unlike PNG files, PDFs are not bound to a specific
resolution. Hence, you can view them at any size without incurring a loss of
quality.

# PDF standards
The International Standards Organization (ISO) has published the base PDF
standard and various standards that extend it to make PDFs more suitable for
specific use-cases. By default, Typst exports PDF 1.7 files. Adobe Acrobat 8 and
later as well as all other commonly used PDF viewers are compatible with this
PDF version.

## PDF/A
Typst optionally supports emitting PDF/A-conformant files. PDF/A files are
geared towards maximum compatibility with current and future PDF tooling. They
do not rely on difficult-to-implement or proprietary features and contain
exhaustive metadata. This makes them suitable for long-term archival.

The PDF/A Standard has multiple versions (_parts_ in ISO terminology) and most
parts have multiple profiles that indicate the file's conformance level.
Currently, Typst supports these PDF/A output profiles:

- PDF/A-2b: The basic conformance level of ISO 19005-2. This version of PDF/A is
  based on PDF 1.7 and results in self-contained, archivable PDF files.

- PDF/A-3b: The basic conformance level of ISO 19005-3. This version of PDF/A is
  based on PDF 1.7 and results in archivable PDF files that can contain
  arbitrary other related files as [attachments]($pdf.embed). The only
  difference between it and PDF/A-2b is the capability to embed
  non-PDF/A-conformant files within.

When choosing between exporting PDF/A and regular PDF, keep in mind that PDF/A
files contain additional metadata, and that some readers will prevent the user
from modifying a PDF/A file. Some features of Typst may be disabled depending on
the PDF standard you choose.

# Exporting as PDF
## Command Line
PDF is Typst's default export format. Running the `compile` or `watch`
subcommand without specifying a format will create a PDF. When exporting to PDF,
you have the following configuration options:

- Which PDF standards Typst should enforce conformance with by specifying
  `--pdf-standard` followed by one or multiple comma-separated standards. Valid
  standards are `1.7`, `a-2b`, and `a-3b`. By default, Typst outputs
  PDF-1.7-compliant files.

- Which pages to export by specifying `--pages` followed by a comma-separated
  list of numbers or dash-separated number ranges. Ranges can be half-open.
  Example: `2,3,7-9,11-`.

## Web App
Click the quick download button at the top right to export a PDF with default
settings. For further configuration, click "File" > "Export as" > "PDF" or click
the downwards-facing arrow next to the quick download button and select "Export
as PDF". When exporting to PDF, you have the following configuration options:

- Which PDF standards Typst should enforce conformance with. By default, Typst
  outputs PDF-1.7-compliant files. Valid additional standards are `A-2b` and
  `A-3b`.

- Which pages to export. Valid options are "All pages", "Current page", and
  "Custom ranges". Custom ranges are a comma-separated list of numbers or
  dash-separated number ranges. Ranges can be half-open. Example: `2,3,7-9,11-`.

# PDF-specific functionality
Typst exposes PDF-specific functionality in the global `pdf` module. See below
for the definitions it contains.
