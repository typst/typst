--- yaml eval ---
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

// Test reading through path type.
#let data-from-path = yaml(path("/assets/data/yaml-types.yaml"))
#test(data-from-path, data)

--- yaml-invalid paged ---
// Error: "/assets/data/bad.yaml" 2:1 failed to parse YAML (did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 18)
#yaml("/assets/data/bad.yaml")

--- yaml-decode-deprecated eval ---
// Warning: 15-21 `yaml.decode` is deprecated, directly pass bytes to `yaml` instead
// Hint: 15-21 it will be removed in Typst 0.15.0
#let _ = yaml.decode

--- yaml-decode-merge-keys eval ---
// This example is copied from https://docs.rs/serde_yaml/0.9.34+deprecated/serde_yaml/enum.Value.html#method.apply_merge. (Apache-2.0 license)
#let config = bytes(
  ```yaml
  tasks:
    build: &webpack_shared
      command: webpack
      args: build
      inputs:
        - 'src/**/*'
    start:
      <<: *webpack_shared
      args: start
  ```.text,
)
#let value = yaml(config, merge-keys: true)
#assert.eq(value.tasks.start.command, "webpack")
#assert.eq(value.tasks.start.args, "start")

// This example is copied from https://yaml.org/type/merge.html. (copyright free)
#let example = bytes(
  ```yaml
  - &CENTER { x: 1, y: 2 }
  - &LEFT { x: 0, y: 2 }
  - &BIG { r: 10 }
  - &SMALL { r: 1 }

  # All the following maps are equal:
  - # Explicit keys
    x: 1
    y: 2
    r: 10
    label: center/big

  - # Merge one map
    << : *CENTER
    r: 10
    label: center/big

  - # Merge multiple maps
    << : [ *CENTER, *BIG ]
    label: center/big

  - # Override
    << : [ *BIG, *LEFT, *SMALL ]
    x: 1
    label: center/big
  ```.text,
)
#let maps = yaml(example, merge-keys: true).slice(-4)
#for m in maps {
  assert.eq(m, maps.first())
}

--- yaml-decode-number eval ---
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

--- yaml-encode-any eval ---
#import "edge-case.typ": special-types-for-human
#for value in special-types-for-human {
  test(
    yaml.encode(value),
    yaml.encode(repr(value)),
  )
}
