// Test representation of values in the document.

---
// Literal values.
{auto} \
{none} (empty) \
{true} \
{false}

---
// Numerical values.
{1} \
{1.0e-4} \
{3.15} \
{1e-10} \
{50.368%} \
{0.0000012345pt} \
{4.5cm} \
{12e1pt} \
{2.5rad} \
{45deg} \
{1.7em} \
{1cm + 0em} \
{2em + 10pt} \
{2.3fr}

---
// Colors.
#rgb("f7a20500") \
{2pt + rgb("f7a20500")}

---
// Strings and escaping.
#repr("hi") \
#repr("a\n[]\"\u{1F680}string")

---
// Content.
#repr[*{"H" + "i"} there*]

---
// Functions
#let f(x) = x

{() => none} \
{f} \
{rect}
