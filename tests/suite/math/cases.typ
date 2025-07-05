// Test case distinction.

--- math-cases ---
$ f(x, y) := cases(
  1 quad &"if" (x dot y)/2 <= 0,
  2 &"if" x divides 2,
  3 &"if" x in NN,
  4 &"else",
) $

--- math-cases-gap ---
#set math.cases(gap: 1em)
$ x = cases(1, 2) $

--- math-cases-delim ---
#set math.cases(delim: sym.angle.l)
$ cases(a, b, c) $

--- math-cases-delim-size ---
// Test setting delimiter size.
$ cases(reverse: #true, 1, 2, 3) cases(delim-size: #100%, 1, 2, 3) $
#set math.cases(delim-size: x => calc.max(x - 5pt, x * 0.901))
$ cases(1, 2) cases(1, 2, 3, 4) $

--- math-cases-linebreaks ---
// Warning: 40-49 linebreaks are ignored in branches
// Hint: 40-49 use commas instead to separate each line
$ cases(a, b, c) cases(reverse: #true, a \ b \ c) $
