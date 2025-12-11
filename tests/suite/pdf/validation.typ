--- pdf-validation-tofu paged pdfstandard(ua-1) ---
// Error: 1-2 PDF/UA-1 error: the text `"ግ"` could not be displayed with font `"Libertinus Serif"`
// Hint: 1-2 try using a different font
ግ

--- pdf-validation-tofu-in-svg paged pdfstandard(ua-1) ---
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
