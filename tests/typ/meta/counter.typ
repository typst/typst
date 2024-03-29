// Test counters.

---
// Count with string key.
#let mine = counter("mine!")

Final: #context mine.final().at(0) \
#mine.step()
First: #context mine.display() \
#mine.update(7)
#context mine.display("1 of 1", both: true) \
#mine.step()
#mine.step()
Second: #context mine.display("I")
#mine.update(n => n * 2)
#mine.step()

---
// Count labels.
#let label = <heya>
#let count = context counter(label).display()
#let elem(it) = [#box(it) #label]

#elem[hey, there!] #count \
#elem[more here!] #count

---
// Count headings.
#set heading(numbering: "1.a.")
#show heading: set text(10pt)
#counter(heading).step()

= Alpha
In #context counter(heading).display()
== Beta

#set heading(numbering: none)
= Gamma
#heading(numbering: "I.")[Delta]

At Beta, it was #context {
  let it = query(heading).find(it => it.body == [Beta])
  numbering(it.numbering, ..counter(heading).at(it.location()))
}

---
// Count figures.
#figure(numbering: "A", caption: [Four 'A's], kind: image, supplement: "Figure")[_AAAA!_]
#figure(numbering: none, caption: [Four 'B's], kind: image, supplement: "Figure")[_BBBB!_]
#figure(caption: [Four 'C's], kind: image, supplement: "Figure")[_CCCC!_]
#counter(figure.where(kind: image)).update(n => n + 3)
#figure(caption: [Four 'D's], kind: image, supplement: "Figure")[_DDDD!_]
