// Test emph and strong.

---
// Basic.
_Emphasized and *strong* words!_

// Inside of a word it's a normal underscore or star.
hello_world Nutzer*innen

// Can contain paragraph in nested content block.
_Still #[

] emphasized._

---
// Inside of words can still use the functions.
P#strong[art]ly em#emph[phas]ized.

---
// Adjusting the delta that strong applies on the weight.
Normal

#set strong(delta: 300)
*Bold*

#set strong(delta: 150)
*Medium* and *#[*Bold*]*

---
// Error: 13 expected underscore
#box[_Scoped] to body.

---
// Ends at paragraph break.
// Error: 7 expected underscore
_Hello

World

---
// Error: 26 expected star
// Error: 26 expected underscore
#[_Cannot *be interleaved]
