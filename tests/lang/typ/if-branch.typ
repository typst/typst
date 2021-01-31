// Test conditions of if-else expressions.

---
// Test condition evaluation.
#if 1 < 2 [
    Ok.
]

#if true == false [
    Bad, but we {dont-care}!
]

---
// Brace in condition.
#if {true} [
    Ok.
]

// Multi-line condition with parens.
#if (
    1 + 1
      == 1
) {
    nope
} #else {
    "Ok."
}

// Multiline.
#if false [
    Bad.
] #else {
    #let pt = "."
    "Ok" + pt
}

---
// Condition must be boolean.
// If it isn't, neither branch is evaluated.
// Error: 5-14 expected boolean, found string
#if "a" + "b" { nope } #else { nope }

// Make sure that we don't complain twice.
// Error: 5-12 cannot add integer and string
#if 1 + "2" {}
