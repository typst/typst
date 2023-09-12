// Test state.

---
#let s = state("hey", "a")
#let double(it) = 2 * it

#s.update(double)
#s.update(double)
$ 2 + 3 $
#s.update(double)

Is: #s.display(),
Was: #locate(location => {
  let it = query(math.equation, location).first()
  s.at(it.location())
}).

---
// Try same key with different initial value.
#state("key", 2).display()
#state("key").update(x => x + 1)
#state("key", 2).display()
#state("key", 3).display()
#state("key").update(x => x + 1)
#state("key", 2).display()

---
#set page(width: 200pt)
#set text(8pt)

#let ls = state("lorem", lorem(1000).split("."))
#let loremum(count) = {
  ls.display(list => list.slice(0, count).join(".").trim() + ".")
  ls.update(list => list.slice(count))
}

#let fs = state("fader", red)
#let trait(title) = block[
  #fs.display(color => text(fill: color)[
    *#title:* #loremum(1)
  ])
  #fs.update(color => color.lighten(30%))
]

#trait[Boldness]
#trait[Adventure]
#trait[Fear]
#trait[Anger]

---
// Make sure that a warning is produced if the layout fails to converge.
// Warning: layout did not converge within 5 attempts
// Hint: check if any states or queries are updating themselves
#let s = state("s", 1)
#locate(loc => s.update(s.final(loc) + 1))
#s.display()
