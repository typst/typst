// Issue #3110: let the error message report the type name.
// https://github.com/typst/typst/issues/3110
// Ref: false

---
// Error: 2-9 type content does not have a constructor
#content()

---
// Error: 6-12 type integer does not contain field `MAXVAL`
#int.MAXVAL

---
// Error: 6-18 type string does not contain field `from-unïcode`
#str.from-unïcode(97)
