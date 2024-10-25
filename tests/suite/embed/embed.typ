// Test embeddings.

--- embed-basic-document- ---
#embed("test.txt")

--- embed-document ---
#embed("test.txt", name: "blub.foo", description: "A test file", mime-type: "text/plain", relationship: "supplement")

--- embed-raw-basic-document ---

#let raw_file = read("test.txt")
#embed.decode(raw_file, "test.txt")

--- embed-raw-document ---

#let raw_file = read("test.txt")
#embed.decode(raw_file, "dir/a_file_name.txt", name: "a_file_name.txt", description: "A description")

--- embed-invalid-relationship ---
// Error: 105-111 expected "source", "data", "alternative", "supplement", "encrypted-payload", "form-data", "schema", "unspecified", or none
#embed("test.txt", name: "blub.foo", description: "A test file", mime-type: "text/plain", relationship: "test")
