// Test representation of values in the document.

#let name = "Typst"
#let ke-bab = "Kebab!"
#let α = "Alpha"

// Variables.
{name} \
{ke-bab} \
{α} \

// Error: 1:2-1:3 unknown variable
{_}

// Literal values.
{none} (empty) \
{true} \
{false} \

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

// Colors.
{#f7a20500} \

// Strings and escaping.
{"hi"} \
{"a\n[]\"\u{1F680}string"} \

// Templates.
{[*{"Hi"} [f 1]*]}
