/// Test scripted warnings (Inspired by https://github.com/typst/typst/issues/1276#issuecomment-1560091418).
// Ref: false

---
#set heading(numbering: "1.")

#let myref(label) = locate(loc => {
    if query(label,loc).len() != 0 {
        ref(label)
    } else {
// Warning: 14-61 could not find reference <test>
        warn("could not find reference <" + str(label) + ">")
    }
})

= Second <test2>

#myref(<test>)

---
// This test verifies warn calls are no-operations if the user defined function calling it is not invoked 
#set heading(numbering: "1.")

#let myref(label) = locate(loc => {
    if query(label,loc).len() != 0 {
        ref(label)
    } else {
        warn("could not find reference <" + str(label) + ">")
    }
})

= Second <test>

---
// This test verifies warn calls are no-operations if the user defined function is called, but the branch of the warn is not hit
#set heading(numbering: "1.")

#let myref(label) = locate(loc => {
    if query(label,loc).len() != 0 {
        ref(label)
    } else {
        warn("could not find reference <" + str(label) + ">")
    }
})

= Header <test>

#myref(<test>)

--- 
#set heading(numbering: "1.")

#let myref(label) = locate(loc => {
    if query(label,loc).len() != 0 {
        ref(label)
    } else {
// Warning: 14-61 could not find reference <test>
// Hint: 14-61 did you mean to add a heading, along the lines of "= Title <test>"?
        warn("could not find reference <" + str(label) + ">", hint: "did you mean to add a heading, along the lines of \"= Title <" + str(label) + ">\"?")
    }
})

= Second <test2>

#myref(<test>)
