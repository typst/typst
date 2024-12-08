// Test embeddings.

--- embed-basic-document ---
#pdf.embed("/assets/text/hello.txt")

--- embed-document ---
#pdf.embed("/assets/text/hello.txt", name: "blub.foo", description: "A test file", mime-type: "text/plain", relationship: "supplement")

--- embed-invalid-relationship ---
// Error: 123-129 expected "source", "data", "alternative", "supplement", "encrypted-payload", "form-data", "schema", "unspecified", or none
#pdf.embed("/assets/text/hello.txt", name: "blub.foo", description: "A test file", mime-type: "text/plain", relationship: "test")

--- embed-raw-basic-document ---
#let raw_file = read("/assets/text/hello.txt")
#pdf.embed.decode(raw_file, "hello.txt")

--- embed-raw-document ---
#let raw_file = read("/assets/text/hello.txt")
#pdf.embed.decode(raw_file, "a_file_name.txt", name: "a_file_name.txt", description: "A description", mime-type: "text/plain", relationship: "supplement")
