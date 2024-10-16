// Test embeddings.

--- basic-document-embedding ---
#embed("test.txt")

--- document-embedding ---
#embed("test.txt", name: "blub.foo", description: "A test file")

--- basic-raw-document-embedding ---

#let raw_file = read("test.txt")
#embed.decode(raw_file, "test.txt")

--- raw-document-embedding ---

#let raw_file = read("test.txt")
#embed.decode(raw_file, "dir/a_file_name.txt", name: "a_file_name.txt", description: "A description")
