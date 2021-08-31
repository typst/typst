// Test representation of values in the document.

---
// Variables.
#let name = "Typst"
#let ke-bab = "Kebab!"
#let α = "Alpha"

{name} \
{ke-bab} \
{α}

---
// Literal values.
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
{45deg}

---
// Colors.
#rgb("f7a20500")

---
// Strings and escaping.
{"hi"} \
{"a\n[]\"\u{1F680}string"}

---
// Templates.
{[*{"H" + "i"} there*]}

---
// Functions
#let f(x) = x

{rect} \
{f} \
{() => none}

---
// When using the `repr` function it's not in monospace.
#repr(23deg)
