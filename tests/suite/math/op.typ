// Test text operators.

--- math-op-predefined ---
// Test predefined.
$ max_(1<=n<=m) n $

--- math-op-call ---
// With or without parens.
$  &sin x + log_2 x \
 = &sin(x) + log_2(x) $

--- math-op-scripts-vs-limits ---
// Test scripts vs limits.
#set page(width: auto)
#set text(font: "New Computer Modern")
Discuss $lim_(n->oo) 1/n$ now.
$ lim_(n->infinity) 1/n = 0 $

--- math-op-custom ---
// Test custom operator.
$ op("myop", limits: #false)_(x:=1) x \
  op("myop", limits: #true)_(x:=1) x $

--- math-op-styled ---
// Test styled operator.
$ bold(op("bold", limits: #true))_x y $

--- math-non-math-content ---
// With non-text content
$ op(#underline[ul]) a $

--- math-op-font ---
// Test with different font.
#let colim = math.op(
  text(font: "IBM Plex Sans", weight: "regular", size: 0.8em)[colim],
  limits: true,
)
$ colim_(x -> 0) inline(colim_(x -> 0)) $

--- math-op-set-font ---
// Test setting font.
#show math.equation: set text(weight: "regular")
#let lig = math.op("fi")
#let test = $sech(x) mod_(x -> oo) lig_1(X)$
#test
#show math.op: set text(font: "New Computer Modern")
#test
