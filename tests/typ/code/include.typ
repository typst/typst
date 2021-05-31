// Test include statements.

---
= Document

// Include a file
#include "importable/chap1.typ"

// The variables of the file should not appear in this scope.
// Error: 1-6 unknown variable
#name

// Expression as a file name.
#let chap2 = include "import" + "able/chap" + "2.typ"

_ -- Intermission -- _
#chap2

{
    // Expressions, code mode.
    // Error: 21-43 file not found
    let x = include "importable/chap3.typ"
}
