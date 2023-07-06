// Test hints on diagnostics.
// Ref: false

---
// Error: 1:17-1:19 expected length, found integer: a length needs a unit - did you mean 12pt?
#set text(size: 12)

---
#{
  let a = 2
  a = 1-a
  a = a -1

  // Error: 7-10 unknown variable: a-1 - if you meant to use subtraction, try adding spaces around the minus sign.
  a = a-1
}

---
#{
  // Error: 3-6 unknown variable: a-1 - if you meant to use subtraction, try adding spaces around the minus sign.
  a-1 = 2
}

---
= Heading <intro>

// Error: 1:20-1:26 cannot reference heading without numbering
// Hint: 1:20-1:26 you can enable heading numbering with `#set heading(numbering: "1.")`
Can not be used as @intro

---
// This test is more of a tooling test. It checks if hint annotation validation
// can be turned off.
// Hints: false

= Heading <intro>

// Error: 1:20-1:26 cannot reference heading without numbering
Can not be used as @intro
