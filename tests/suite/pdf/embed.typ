// Test embeddings.

--- pdf-embed ---
#pdf.embed("/assets/text/hello.txt")
#pdf.embed(
  "/assets/data/details.toml",
  relationship: "supplement",
  mime-type: "application/toml",
  description: "Information about a secret project",
)

--- pdf-embed-invalid-relationship ---
#pdf.embed(
  "/assets/text/hello.txt",
  // Error: 17-23 expected "source", "data", "alternative", "supplement", or none
  relationship: "test",
  mime-type: "text/plain",
  description: "A test file",
)

--- pdf-embed-decode ---
#pdf.embed.decode("hello.txt", read("/assets/text/hello.txt"))
#pdf.embed.decode(
  "a_file_name.txt",
  read("/assets/text/hello.txt"),
  relationship: "supplement",
  mime-type: "text/plain",
  description: "A description",
)
