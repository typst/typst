// Test the column layouter.

---
// Test normal operation and RTL directions.
#set page(height: 3.25cm, width: 7.05cm, columns: 2, column-gutter: 30pt)
#set text("Noto Sans Arabic", serif)
#set par(lang: "ar")

#rect(fill: conifer, height: 8pt, width: 6pt) وتحفيز
العديد من التفاعلات الكيميائية. (DNA) من أهم الأحماض النووية التي تُشكِّل
إلى جانب كل من البروتينات والليبيدات والسكريات المتعددة
#rect(fill: eastern, height: 8pt, width: 6pt)
الجزيئات الضخمة الأربعة الضرورية للحياة. 

---
// Test the `columns` function.
#set page(width: auto)

#rect(width: 180pt, height: 100pt, padding: 8pt, columns(2, [
    A special plight has befallen our document.
    Columns in text boxes reigned down unto the soil
    to waste a year's crop of rich layouts.
    The columns at least were graciously balanced.
]))

---
// Test columns for a sized page.
#set page(height: 5cm, width: 7.05cm, columns: 2)

Lorem ipsum dolor sit amet is a common blind text
and I again am in need of filling up this page 
#align(bottom, rect(fill: eastern, width: 100%, height: 12pt))
#colbreak()

so I'm returning to this trusty tool of tangible terror.
Sure, it is not the most creative way of filling up
a page for a test but it does get the job done.

---
// Test the expansion behavior.
#set page(height: 2.5cm, width: 7.05cm)

#rect(padding: 6pt, columns(2, [
    ABC \
    BCD
    #colbreak()
    DEF
]))

---
// Test setting a column gutter and more than two columns.
#set page(height: 3.25cm, width: 7.05cm, columns: 3, column-gutter: 30pt)

#rect(width: 100%, height: 2.5cm, fill: conifer)
#rect(width: 100%, height: 2cm, fill: eastern)
#circle(fill: eastern)

---
// Test the `colbreak` and `pagebreak` functions.
#set page(height: 1cm, width: 7.05cm, columns: 2)

A
#colbreak()
#colbreak()
B
#pagebreak()
C
#colbreak()
D

---
// Test an empty second column.
#set page(width: 7.05cm, columns: 2)

#rect(width: 100%, padding: 3pt)[So there isn't anything in the second column?]

---
// Test columns when one of them is empty.
#set page(width: auto, columns: 3)

Arbitrary horizontal growth.

---
// Test columns in an infinitely wide frame.
#set page(width: 7.05cm, columns: 2)

There can be as much content as you want in the left column
and the document will grow with it.

#rect(fill: conifer, width: 100%, height: 30pt)

Only an explicit #colbreak() `#colbreak()` can put content in the
second column.

---
// Test a page with a single column.
#set page(height: auto, width: 7.05cm, columns: 1)

This is a normal page. Very normal.

---
// Test a page with zero columns.
// Error: 49-50 must be positive
#set page(height: auto, width: 7.05cm, columns: 0)

This makes less sense.

// colbreak in auto stroke box on sized page that should be higher than box