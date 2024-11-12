// Test embeddings.

--- embed-basic-document- ---
#pdf.embed("test.txt")

--- embed-document ---
#pdf.embed("test.txt", name: "blub.foo", description: "A test file", mime-type: "text/plain", relationship: "supplement")

--- embed-invalid-relationship ---
// Error: 109-115 expected "source", "data", "alternative", "supplement", "encrypted-payload", "form-data", "schema", "unspecified", or none
#pdf.embed("test.txt", name: "blub.foo", description: "A test file", mime-type: "text/plain", relationship: "test")

--- embed-raw-basic-document ---
#let raw_file = read("test.txt")
#pdf.embed.decode(raw_file, "test.txt")

--- embed-raw-document ---
#let raw_file = read("test.txt")
#pdf.embed.decode(raw_file, "dir/a_file_name.txt", name: "a_file_name.txt", description: "A description", mime-type: "text/plain", relationship: "supplement")
