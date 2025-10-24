// Test file attachments. The tests here so far are unsatisfactory because we
// have no PDF testing infrastructure. That should be improved in the future.

--- pdf-attach render ---
#pdf.attach("/assets/text/hello.txt")
#pdf.attach(
  "/assets/data/details.toml",
  relationship: "supplement",
  mime-type: "application/toml",
  description: "Information about a secret project",
)

--- pdf-attach-bytes render ---
#pdf.attach("hello.txt", read("/assets/text/hello.txt", encoding: none))
#pdf.attach(
  "a_file_name.txt",
  read("/assets/text/hello.txt", encoding: none),
  relationship: "supplement",
  mime-type: "text/plain",
  description: "A description",
)

--- pdf-attach-invalid-relationship render ---
#pdf.attach(
  "/assets/text/hello.txt",
  // Error: 17-23 expected "source", "data", "alternative", "supplement", or none
  relationship: "test",
  mime-type: "text/plain",
  description: "A test file",
)

--- pdf-attach-invalid-data render ---
// Error: 39-46 expected bytes, found string
#pdf.attach("/assets/text/hello.txt", "hello")

--- pdf-embed-deprecated render ---
// Warning: 6-11 the name `embed` is deprecated, use `attach` instead
// Hint: 6-11 it will be removed in Typst 0.15.0
#pdf.embed("/assets/text/hello.txt")
