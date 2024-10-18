// Test embeddings.

--- embed-basic-document- ---
#embed("test.txt")

--- embed-document ---
#embed("test.txt", name: "blub.foo", description: "A test file")

--- embed-raw-basic-document ---

#let raw_file = read("test.txt")
#embed.decode(raw_file, "test.txt")

--- embed-raw-document ---

#let raw_file = read("test.txt")
#embed.decode(raw_file, "dir/a_file_name.txt", name: "a_file_name.txt", description: "A description")
