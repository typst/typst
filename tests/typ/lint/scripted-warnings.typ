// Test script-emitted warnings.
// Ref: false

---
// Example inspired by https://github.com/typst/typst/issues/1276#issuecomment-1560091418
#set heading(numbering: "1.")

#let myref(label) = locate(loc => {
  if query(label,loc).len() != 0 {
    ref(label)
  } else {
    // Warning: 10-57 could not find reference <test>
    warn("could not find reference <" + str(label) + ">")
  }
})

= Second <test2>

#myref(<test>)

---
// This test verifies warn calls are not invoked if the user defined function calling it is not invoked 
#let someFunc() = {
  warn("this should not push a warning into the diagnostics")
}

---
// This test verifies warn calls are not invoked if the user defined function is called, but the branch of the warn is not hit
#let someFunc() = {
  if false {
    warn("this branch should not be hit, so no diagnostic should be emitted")
  } else {
    [this is fine]
  }
}

#someFunc()

--- 
#let warningWithHint() = {
  // Warning: 8-29 this is the warning
  // Hint: 8-29 this is the hint
  warn("this is the warning", hint: "this is the hint")
}

#warningWithHint()

---
// Warning: 7-40 did you misconfigure something?
#warn("did you misconfigure something?")
