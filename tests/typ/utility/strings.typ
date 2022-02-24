// Test string functions.

---
// Test the `upper`, `lower`, and number formatting functions.
#upper("Abc 8")
#upper[def]

#lower("SCREAMING MUST BE SILENCED in " + roman(1672) + " years")

#for i in range(9) {
    symbol(i)
    [ and ]
    roman(i)
    [ for #i]
    parbreak()
}

---
// Error: 8-9 expected string or template, found integer
#upper(1)

---
// Error: 9-11 must be at least zero
#symbol(-1)
