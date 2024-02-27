// Tests content field access.

---
// Ensure that fields from set rules are materialized into the element before
// a show rule runs.
#set table(columns: (10pt, auto))
#show table: it => it.columns
#table[A][B][C][D]

---
// Test it again with a different element.
#set heading(numbering: "(I)")
#show heading: set text(size: 11pt, weight: "regular")
#show heading: it => it.numbering
= Heading

---
// Test it with query.
#set raw(lang: "rust")
#context query(<myraw>).first().lang
`raw` <myraw>

---
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
