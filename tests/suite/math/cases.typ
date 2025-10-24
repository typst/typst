// Test case distinction.

--- math-cases render ---
$ f(x, y) := cases(
  1 quad &"if" (x dot y)/2 <= 0,
  2 &"if" x divides 2,
  3 &"if" x in NN,
  4 &"else",
) $

--- math-cases-gap render ---
#set math.cases(gap: 1em)
$ x = cases(1, 2) $

--- math-cases-delim render ---
#set math.cases(delim: sym.chevron.l)
$ cases(a, b, c) $

--- math-cases-linebreaks render ---
// Warning: 40-49 linebreaks are ignored in branches
// Hint: 40-49 use commas instead to separate each line
$ cases(a, b, c) cases(reverse: #true, a \ b \ c) $
