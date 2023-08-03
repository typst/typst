//Tests for math ($...$) as an extended argument to a function call

---
// Basics
#let double(x) = {x; x}
#double$a=b$
#double$ a != b $

---
// Doesn't work in math, and only works for one argument.
#let double(x) = {x; x}
$#double("x")$text$y$
#double$x$$y$

---
// Works in code, and in markup in code
#let bookend(x,y) = { x; y; x}
#{ bookend("a")$b$ }
#{ [ #bookend("a")$b$ ] }

---
// Error: 2:15-2:20 missing argument: y
#let bookend(x,y) = { x; y; x}
#{ [ $#bookend("a")$ ] }

---
// Access field afterwards.
#let echo(x) = { x }
#echo$ w $.block

---
// Call function afterwards.
#let hspace(x) = { h }
x#hspace$ w $(2em)y

---
// It ends a statment.
#let double(x) = {x;x}
#double$w$ is fine inline.

---
// It works ok with unary and binary operators.
#let five(x) = {5}
#{ -five()$ w $ } #{five$ w $ + five$ z $}
//#-five()

--- 
// It is repeatable if interruped by a call.
#let double(x) = {x;x}
#let mkdouble(y) = {double}
#mkdouble$x$()$y$