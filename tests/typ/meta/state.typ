// Test state.

---
#set page(width: 200pt)
#set text(8pt)

#let ls = state("lorem", lorem(1000).split("."))
#let loremum(count) = {
  ls.get(list => list.slice(0, count).join(".").trim() + ".")
  ls.update(list => list.slice(count))
}

#let fs = state("fader", red)
#let trait(title) = block[
  #fs.get(color => text(fill: color)[
    *#title:* #loremum(1)
  ])
  #fs.update(color => color.lighten(30%))
]

#trait[Boldness]
#trait[Adventure]
#trait[Fear]
#trait[Anger]
