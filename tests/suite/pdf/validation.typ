--- pdf-validation-multiple-standards pdf pdfstandard(a-2a, ua-1) ---
#set document(date: datetime(year: 1970, month: 1, day: 1))
Hello

--- pdf-validation-version-incompatible pdf pdfstandard(1.4, a-4) ---
// Error: PDF 1.4 is not compatible with PDF/A-4
// Hint: PDF/A-4 requires version PDF 2.0
Hello

--- pdf-validation-standards-incompatible pdf pdfstandard(ua-1, a-4) ---
// Error: PDF/A-4 and PDF/UA-1 are mutually incompatible because they do not have any overlapping PDF versions
// Hint: PDF/A-4 requires version PDF 2.0
// Hint: PDF/UA-1 requires a version between PDF 1.4 and PDF 1.7
Hello

--- pdf-validation-min-feature-version-unsatisfiable pdf pdfstandard(ua-1, a-1a) ---
#set document(date: datetime(year: 1970, month: 1, day: 1))
#table(
  // Error: 3-37 PDF/UA-1 error: table header cell cannot be accessibly tagged in PDF 1.4 files
  // Hint: 3-37 PDF version must be at least PDF 1.5 to satisfy PDF/UA-1
  // Hint: 3-37 remove or replace the PDF/A-1a standard, as it prevents PDF versions beyond PDF 1.4
  pdf.header-cell(scope: "row")[brr]
)

--- pdf-validation-missing-feature-single-version pdf pdfstandard(1.4, ua-1) ---
// Error: PDF/UA-1 error: headers and footers cannot be made accessible in PDF 1.4 files
// Hint: set the version to PDF 1.7
#set page(header: [Hi])

--- pdf-validation-missing-feature-multiple-versions pdf pdfstandard(1.4, ua-1) ---
// Error: PDF/UA-1 error: links and other annotations cannot be navigated accessibly in PDF 1.4 files
// Hint: select a version between PDF 1.5 and PDF 1.7
https://typst.app/

--- pdf-validation-tofu pdf pdfstandard(ua-1) ---
// Error: 1-2 PDF/UA-1 error: the text `"ግ"` could not be displayed with font `"Libertinus Serif"`
// Hint: 1-2 try using a different font
ግ

--- pdf-validation-tofu-in-svg pdf pdfstandard(ua-1) ---
// A spanless error without a font name is kinda bad, but this used to be a
// crash, so it's already an improvement.

// Error: PDF/UA-1 error: the text `"ግ"` could not be displayed with a font
// Hint: try using a different font
#image(bytes(
  ```
  <?xml version="1.0" encoding="utf-8"?>
  <svg width="10" height="10" xmlns="http://www.w3.org/2000/svg">
    <text>ግ</text>
  </svg>
  ```.text
), alt: "Geʽez letter")

--- pdf-validation-bundle bundle pdfstandard(ua-1) ---
#document(
  "hi.pdf",
  // Error: 13-28 PDF/UA-1 error: cannot combine underline, overline, or strike
  underline(overline[Hello]),
)
