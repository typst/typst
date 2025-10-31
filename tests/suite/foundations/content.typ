--- content-at-default paged ---
// Test .at() default values for content.
#test(auto, [a].at("doesn't exist", default: auto))

--- content-field-syntax render ---
// Test fields on elements.
#show list: it => {
  test(it.children.len(), 3)
}

- A
- B
- C

--- content-field-missing paged diagnostic ---
// Error: 25-28 heading does not have field "fun"
#show heading: it => it.fun
= A

--- content-fields paged ---
// Test content fields method.
#test([a].fields(), (text: "a"))
#test([a *b*].fields(),  (children: ([a], [ ], strong[b])))

--- content-fields-mutable-invalid paged diagnostic ---
#{
  let object = [hi]
  // Error: 3-9 cannot mutate fields on content
  object.property = "value"
}

--- content-field-materialized-table render ---
// Ensure that fields from set rules are materialized into the element before
// a show rule runs.
#set table(columns: (10pt, auto))
#show table: it => it.columns
#table[A][B][C][D]

--- content-field-materialized-heading render ---
// Test it again with a different element.
#set heading(numbering: "(I)")
#show heading: set text(size: 11pt, weight: "regular")
#show heading: it => it.numbering
= Heading

--- content-field-materialized-query render ---
// Test it with query.
#set raw(lang: "rust")
#context query(<myraw>).first().lang
`raw` <myraw>

--- content-fields-complex render ---
// Integrated test for content fields. The idea is to parse a normal looking
// equation and symbolically evaluate it with the given variable values.

#let compute(equation, ..vars) = {
  let vars = vars.named()
  let f(elem) = {
    let func = elem.func()
    if elem.has("text") {
      let text = elem.text
      if regex("^\d+$") in text {
        int(text)
      } else if text in vars {
        int(vars.at(text))
      } else {
        panic("unknown math variable: " + text)
      }
    } else if func == math.attach {
      let value = f(elem.base)
      if elem.has("t") {
        value = calc.pow(value, f(elem.t))
      }
      value
    } else if elem.has("children") {
      elem
        .children
        .filter(v => v != [ ])
        .split($+$.body)
        .map(xs => xs.fold(1, (prod, v) => prod * f(v)))
        .fold(0, (sum, v) => sum + v)
    }
  }
  let result = f(equation.body)
  [With ]
  vars
    .pairs()
    .map(((name, value)) => $#symbol(name) = value$)
    .join(", ", last: " and ")
  [ we have:]
  $ equation = result $
}

#compute($x y + y^2$, x: 2, y: 3)
// This should generate the same output as:
// With $x = 2$ and $y = 3$ we have: $ x y + y^2 = 15 $

--- content-label-has-method render ---
// Test whether the label is accessible through the `has` method.
#show heading: it => {
  assert(it.has("label"))
  it
}

= Hello, world! <my-label>

--- content-label-field-access render ---
// Test whether the label is accessible through field syntax.
#show heading: it => {
  assert(str(it.label) == "my-label")
  it
}

= Hello, world! <my-label>

--- content-label-fields-method render ---
// Test whether the label is accessible through the fields method.
#show heading: it => {
  assert("label" in it.fields())
  assert(str(it.fields().label) == "my-label")
  it
}

= Hello, world! <my-label>

--- content-fields-unset paged diagnostic ---
// Error: 10-15 field "block" in raw is not known at this point
#raw("").block

--- content-fields-unset-no-default paged diagnostic ---
// Error: 2-21 field "block" in raw is not known at this point and no default was specified
#raw("").at("block")

--- content-try-to-access-internal-field paged diagnostic ---
// Error: 9-15 hide does not have field "hidden"
#hide[].hidden
