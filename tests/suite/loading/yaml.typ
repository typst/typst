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
// Error: "/assets/data/bad.yaml" 2:1 failed to parse YAML (did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 18)
#yaml("/assets/data/bad.yaml")

--- yaml-decode-deprecated ---
// Warning: 15-21 `yaml.decode` is deprecated, directly pass bytes to `yaml` instead
// Hint: 15-21 it will be removed in Typst 0.15.0
#let _ = yaml.decode

--- yaml-encode-any ---
// Anything can be encoded.
// Unsupported types fall back to strings via `repr`, but the specific output can be changed.
#let check(value) = test(
  yaml.encode(value),
  yaml.encode(repr(value)),
)

#check(bytes("Typst"))
#check(decimal("2.99792458"))
#check(3.14159pt)
#check(2.99em)
#check(<sec:intro>)
#check(panic)
#check(x => x + 1)
#check(regex("\p{Letter}"))
#check(stroke(red + 5pt))
#check(auto)
