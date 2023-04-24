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
