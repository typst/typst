// Test include statements.

---
#page(width: 200pt)

= Document

// Include a file
#include "importable/chap1.typ"

// Expression as a file name.
#let chap2 = include "import" + "able/chap" + "2.typ"

-- _Intermission_ --
#chap2

---
{
  // Error: 19-41 file not found
  let x = include "importable/chap3.typ"
}

---
#include "importable/chap1.typ"

// The variables of the file should not appear in this scope.
// Error: 1-6 unknown variable
#name
