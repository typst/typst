// Test that a space after a named parameter is permissible.
// https://github.com/typst/typst/issues/3502
// Ref: false

---
#let f( param : v ) = param
#test(f( param /* ok */ : 2 ), 2)

---
#let ( key :  /* hi */ binding ) = ( key: "ok" )
#test(binding, "ok")

---
#test(( key : "value" ).key, "value")
