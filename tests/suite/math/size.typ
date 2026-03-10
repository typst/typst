--- math-size paged ---
// Test forcing math size
$a/b, display(a/b), display(a)/display(b), inline(a/b), script(a/b), sscript(a/b) \
 mono(script(a/b)), script(mono(a/b))\
 script(a^b, cramped: #true), script(a^b, cramped: #false)$

--- issue-3658-math-size paged ---
$ #rect[$1/2$] $
$#rect[$1/2$]$
