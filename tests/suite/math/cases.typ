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

--- math-cases-linebreaks ---
// Currently linebreaks are equivalent to commas, though this behaviour may
// change in the future.
$ cases(a, b, c) cases(reverse: #true, a \ b \ c) $
