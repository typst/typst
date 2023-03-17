// Test counters.

---
// Count with string key.
#let mine = counter("mine!")

Final: #mine.final() \
#mine.step()
#mine.step()
First: #mine.get() \
#mine.update(7)
#mine.both("1 of 1") \
#mine.step()
#mine.step()
Second: #mine.get("I")
#mine.update(n => n * 2)
#mine.step()

---
// Count labels.
#let label = <heya>
#let count = counter(label).get()
#let elem(it) = [#box(it) #label]

#elem[hey, there!] #count \
#elem[more here!] #count

---
// Count headings.
#set heading(numbering: "1.a.")
#show heading: set text(10pt)
#counter(heading).step()

= Alpha
== Beta
In #counter(heading).get().

#set heading(numbering: none)
= Gamma
#heading(numbering: "I.")[Delta]

---
// Count figures.
#figure(numbering: "A", caption: [Four 'A's])[_AAAA!_]
#figure(numbering: none, caption: [Four 'B's])[_BBBB!_]
#figure(caption: [Four 'C's])[_CCCC!_]
#counter(figure).update(n => n + 3)
#figure(caption: [Four 'D's])[_DDDD!_]
