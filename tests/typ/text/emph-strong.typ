// Test emph and strong.

---
// Basic.
_Emphasized and *strong* words!_

// Inside of a word it's a normal underscore or star.
hello_world Nutzer*innen

// Can contain paragraph in child template.
_Still [

] emphasized._

---
// Inside of words can still use the functions.
P#strong[art]ly em#emph[phas]ized.

---
// Error: 13 expected underscore
#box[_Scoped] to body.

---
// Ends at paragraph break.
// Error: 7 expected underscore
_Hello

World

---
// Error: 1:12 expected star
// Error: 2:1 expected star
_Cannot *be_ interleaved*
