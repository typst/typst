/// Test scripted warnings (Inspired by https://github.com/typst/typst/issues/1276#issuecomment-1560091418).
// Ref: false

---
#set heading(numbering: "1.")

#let myref(label) = locate(loc =>{
    if query(label,loc).len() != 0 {
        ref(label)
    } else {
// Warning: 14-61 Could not find reference <test>
        warn("Could not find reference <" + str(label) + ">")
    }
})

= Second <test2>

#myref(<test>)
