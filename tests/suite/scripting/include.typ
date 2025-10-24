// Test module includes.

--- include-file paged ---
#set page(width: 200pt)

// Include a file
#include "modules/chap1.typ"

// Expression as a file name.
#let chap2 = include "modu" + "les/chap" + "2.typ"

-- _Intermission_ --
#chap2

--- include-file-not-found paged ---
#{
  // Error: 19-38 file not found (searched at tests/suite/scripting/modules/chap3.typ)
  let x = include "modules/chap3.typ"
}

--- include-no-bindings paged ---
#include "modules/chap1.typ"

// The variables of the file should not appear in this scope.
// Error: 2-6 unknown variable: name
#name

--- include-semicolon-or-linebreak paged ---
// Error: 18 expected semicolon or line break
#include "hi.typ" Hi
