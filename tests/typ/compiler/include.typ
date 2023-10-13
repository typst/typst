// Test module includes.

---
#set page(width: 200pt)

= Document

// Include a file
#include "modules/chap1.typ"

// Expression as a file name.
#let chap2 = include "modu" + "les/chap" + "2.typ"

-- _Intermission_ --
#chap2

---
#{
  // Error: 19-38 file not found (searched at typ/compiler/modules/chap3.typ)
  let x = include "modules/chap3.typ"
}

---
#include "modules/chap1.typ"

// The variables of the file should not appear in this scope.
// Error: 2-6 unknown variable: name
#name

---
// Error: 18 expected semicolon or line break
#include "hi.typ" Hi
