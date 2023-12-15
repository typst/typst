// https://github.com/typst/typst/issues/2214
// The math content should also be affected by the TextElem baseline.

hello #text(baseline: -5pt)[123 #sym.WW\orld]\
hello #text(baseline: -5pt)[$123 WW#text[or]$ld]\
