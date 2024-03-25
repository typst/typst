// Test math-mode function call parsing and structure.

---
// Test function calls that aren't typst functions
$ pi(a) $
$ pi(a,) $
$ pi(a,b) $
$ pi(a,b,) $

---
// Test 2-d arguments with whitespace/trivia between commas.
$ mat(;,) $ // this one is fine
// Error: 8-8 expected array, found content
$ mat(; ,) $
$ mat(;/**/,) $
$ mat(;
,) $
$ mat(
  1, , ;
   ,1, ;
   , ,1;
) $

---
// Test 2-d argument structure.
#set page(width: auto)
#let func(..body) = body;
$ func( a; b; ) $
$ func(a;  ; c) $
$ func(a b,/**/; b) $
$ func(a/**/b, ; b) $
$ func( ;/**/a/**/b/**/; ) $
$ func( ; , ; ) $
$ func(/**/; // funky whitespace/trivia
    ,   /**/  ;/**/) $

---
// Error: 6-7 expected content, found array
// Error: 8-9 expected content, found array
$ pi(a;b) $

---
// Test functions with trailing commas with and without whitespace.
#let func(..body) = body
$ sin(x, y,) $
$ sin(x,y,,,) $
$ sin(/**/x/**/, /**/y, ,/**/, ) $
$ func(x, y,) $
$ func(x,y,,,) $
$ func(/**/x/**/, /**/y, ,/**/, ) $
