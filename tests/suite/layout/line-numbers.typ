--- line-numbers-enable ---
#set page(margin: (left: 1.5cm))
#set par.line(numbering: "1")

First line \
Second line \
Third line

--- line-numbers-clearance ---
#set page(margin: (left: 1.5cm))
#set par.line(numbering: "1", number-clearance: 0cm)

First line \
Second line \
Third line

--- line-numbers-margin ---
#set page(margin: (right: 3cm))
#set par.line(numbering: "1", number-clearance: 1.5cm, number-margin: end)

First line \
Second line \
Third line

--- line-numbers-default-alignment ---
#set page(margin: (left: 2cm))
#set par.line(numbering: "1")
a
#([\ a] * 15)

--- line-numbers-start-alignment ---
#set page(margin: (left: 2cm))
#set par.line(numbering: "i", number-align: start)
a \
a
#pagebreak()
a \
a \
a

--- line-numbers-rtl ---
#set page(margin: (right: 2cm))
#set text(dir: rtl)
#set par.line(numbering: "1")
a
#([\ a] * 15)

--- line-numbers-columns ---
#set page(columns: 2, margin: (x: 1.5em))
#set par.line(numbering: "1", number-clearance: 0.5em)

Hello \
Beautiful \
World
#colbreak()
Birds\
In the\
Sky

--- line-numbers-multi-columns ---
#set page(columns: 3, margin: (x: 1.5em))
#set par.line(numbering: "1", number-clearance: 0.5em)

A\
B\
C
#colbreak()
D\
E\
F
#colbreak()
G\
H\
I

--- line-numbers-columns-rtl ---
#set page(columns: 2, margin: (x: 1.5em))
#set par.line(numbering: "1", number-clearance: 0.5em)
#set text(dir: rtl)

Hello \
Beautiful \
World
#colbreak()
Birds\
In the\
Sky

--- line-numbers-columns-override ---
#set columns(gutter: 1.5em)
#set page(columns: 2, margin: (x: 1.5em))
#set par.line(numbering: "1", number-margin: end, number-clearance: 0.5em)

Hello \
Beautiful \
World
#colbreak()
Birds\
In the\
Sky
