--- yaml ---
// Test reading YAML data
#let data = yaml("/assets/data/yaml-types.yaml")
#test(data.len(), 9)
#test(data.null_key, (none, none))
#test(data.string, "text")
#test(data.integer, 5)
#test(data.float, 1.12)
#test(data.mapping, ("1": "one", "2": "two"))
#test(data.seq, (1, 2, 3, 4))
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

--- yaml-decode-number ---
#import "edge-case.typ": large-integer, representable-integer

#for (name, source) in representable-integer {
  assert.eq(
    type(yaml(bytes(source))),
    int,
    message: "failed to decode " + name,
  )
}

#for (name, source) in large-integer {
  assert.eq(
    type(yaml(bytes(source))),
    float,
    message: "failed to approximately decode " + name,
  )
}
