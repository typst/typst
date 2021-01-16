#let name = "Typst";
#let ke-bab = "Kebab!";
#let α = "Alpha";

{name} \
{ke-bab} \
{α} \
{none} (empty) \
{true} \
{false} \
{1.0e-4} \
{3.15} \
{1e-10} \
{50.368%} \
{0.0000012345pt} \
{4.5cm} \
{12e1pt} \
{2.5rad} \
{45deg} \
{"hi"} \
{"a\n[]\"\u{1F680}string"} \
{#f7a20500} \
{[*{"Hi"} [f 1]*]} \
{{1}}

// Error: 1:1-1:4 unknown variable
{_} \

// Error: 1:2-1:5 invalid color
{#a5}

// Error: 1:2-1:4 expected expression, found invalid token
{1u}
