--- content-at-default ---
// Test .at() default values for content.
#test(auto, [a].at("doesn't exist", default: auto))

--- content-field-syntax ---
// Test fields on elements.
#show list: it => {
  test(it.children.len(), 3)
}

- A
- B
- C

--- content-field-missing ---
// Error: 25-28 heading does not have field "fun"
#show heading: it => it.fun
= A

--- content-fields ---
// Test content fields method.
#test([a].fields(), (text: "a"))
#test([a *b*].fields(),  (children: ([a], [ ], strong[b])))

--- content-fields-mutable-invalid ---
#{
  let object = [hi]
  // Error: 3-9 cannot mutate fields on content
  object.property = "value"
}

--- content-field-materialized-table ---
// Ensure that fields from set rules are materialized into the element before
// a show rule runs.
#set table(columns: (10pt, auto))
#show table: it => it.columns
#table[A][B][C][D]

--- content-field-materialized-heading ---
// Test it again with a different element.
#set heading(numbering: "(I)")
#show heading: set text(size: 11pt, weight: "regular")
#show heading: it => it.numbering
= Heading

--- content-field-materialized-query ---
// Test it with query.
#set raw(lang: "rust")
#context query(<myraw>).first().lang
`raw` <myraw>

--- content-fields-complex ---
// Integrated test for content fields.
#let compute(equation, ..vars) = {
  let vars = vars.named()
  let f(elem) = {
    let func = elem.func()
    if func == text {
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
        .split[+]
        .map(xs => xs.fold(1, (prod, v) => prod * f(v)))
        .fold(0, (sum, v) => sum + v)
    }
  }
  let result = f(equation.body)
  [With ]
  vars
    .pairs()
    .map(p => $#p.first() = #p.last()$)
    .join(", ", last: " and ")
  [ we have:]
  $ equation = result $
}

#compute($x y + y^2$, x: 2, y: 3)

--- content-label-has-method ---
// Test whether the label is accessible through the `has` method.
#show heading: it => {
  assert(it.has("label"))
  it
}

= Hello, world! <my-label>

--- content-label-field-access ---
// Test whether the label is accessible through field syntax.
#show heading: it => {
  assert(str(it.label) == "my-label")
  it
}

= Hello, world! <my-label>

--- content-label-fields-method ---
// Test whether the label is accessible through the fields method.
#show heading: it => {
  assert("label" in it.fields())
  assert(str(it.fields().label) == "my-label")
  it
}

= Hello, world! <my-label>

--- content-fields-unset ---
// Error: 10-15 field "block" in raw is not known at this point
#raw("").block

--- content-fields-unset-no-default ---
// Error: 2-21 field "block" in raw is not known at this point and no default was specified
#raw("").at("block")

--- content-try-to-access-internal-field ---
// Error: 9-15 hide does not have field "hidden"
#hide[].hidden
