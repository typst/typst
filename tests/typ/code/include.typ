// Test module includes.

---
#set page(width: 200pt)

= Document

// Include a file
#include "/typ/code/importable/chap1.typ"

// Expression as a file name.
#let chap2 = include "import" + "able/chap" + "2.typ"

-- _Intermission_ --
#chap2

---
{
  // Error: 19-41 file not found (searched at typ/code/importable/chap3.typ)
  let x = include "importable/chap3.typ"
}

---
#include "importable/chap1.typ"

// The variables of the file should not appear in this scope.
// Error: 1-6 unknown variable
#name

---
// Error: 18 expected semicolon or line break
#include "hi.typ" Hi
