// Integrated test for content fields.

#let compute(equation, ..vars) = {
  let vars = vars.named()
  let f(node) = {
    let func = node.func()
    if func == text {
      let text = node.text
      if regex("^\d+$") in text {
        int(text)
      } else if text in vars {
        int(vars.at(text))
      } else {
        panic("unknown math variable: " + text)
      }
    } else if func == math.attach {
      let value = f(node.base)
      if node.has("top") {
        value = calc.pow(value, f(node.top))
      }
      value
    } else if node.has("children") {
      node
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
