--- yaml ---
// Test reading YAML data
#let data = yaml("/assets/data/yaml-types.yaml")
#test(data.len(), 9)
#test(data.null_key, (none, none))
#test(data.string, "text")
#test(data.integer, 5)
#test(data.float, 1.12)
#test(data.mapping, ("1": "one", "2": "two"))
#test(data.seq, (1,2,3,4))
#test(data.bool, false)
#test(data.keys().contains("true"), true)
#test(data.at("1"), "ok")

--- yaml-invalid ---
// Error: 7-30 failed to parse YAML (did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 18)
#yaml("/assets/data/bad.yaml")
