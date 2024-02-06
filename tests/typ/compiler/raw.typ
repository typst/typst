// Test new raw parser
// Ref: false

---
#let empty = (
  name: "empty",
  input: ``,
  text: "",
)

#let backtick = (
  name: "backtick",
  input: ``` ` ```,
  text: "`",
  block: false,
)

#let lang-backtick = (
  name: "lang-backtick",
  input: ```js ` ```,
  lang: "js",
  text: "`",
  block: false,
)

#let lang-space = (
  name: "lang-space",
  input: ```js test ```,
  lang: "js",
  text: "test ",
  block: false,
)

#let lang-newline = (
  name: "lang-newline",
  input: ```js
test
```,
  lang: "js",
  text: "test",
  block: true,
)

#let blocky = (
  name: "blocky",
  input: {
```
test
```
},
  text: "test",
  block: true,
)

#let blocky-dedent = (
  name: "blocky-dedent",
  input: {
```
 test
 ```
  },
  text: "test",
  block: true,
)

#let blocky-dedent-dont-considering-first-line = (
  name: "blocky-dedent-dont-considering-first-line",
  input: {
```      
 test
 ```
  },
  text: "test",
  block: true,
)

#let blocky-dedent-lastline = (
  name: "blocky-dedent-lastline",
  input: ```
  test
 ```,
  text: " test",
  block: true,
)

#let blocky-dedent-lastline2 = (
  name: "blocky-dedent-lastline2",
  input: ```
  test
   ```,
  text: "test",
  block: true,
)

#let blocky-tab = (
  name: "blocky-tab",
  input: {
```
	test
```
},
  text: "\ttest",
  block: true,
)

#let blocky-tab-dedent = (
  name: "blocky-tab-dedent",
  input: {
```
	test
  
 ```
},
  text: "test\n ",
  block: true,
)

#let cases = (
  empty,
  backtick,
  lang-backtick,
  lang-space,
  lang-newline,
  blocky,
  blocky-dedent-dont-considering-first-line,
  blocky-dedent,
  blocky-dedent-lastline,
  blocky-dedent-lastline2,
  blocky-tab,
  blocky-tab-dedent,
)

#for c in cases {
  assert.eq(c.text, c.input.text, message: "in point " + c.name + ", expect " + repr(c.text) + ", got " + repr(c.input.text) + "")
  let block = c.at("block", default: false)
  assert.eq(block, c.input.block, message: "in point " + c.name + ", expect " + repr(block) + ", got " + repr(c.input.block) + "")
}
