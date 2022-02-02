// Test string functions.

---
// Test the `upper`, `lower`, and number formatting functions. 
#upper("Abc 8 def")

#lower("SCREAMING MUST BE SILENCED in " + roman(1672) + " years")

#for i in range(9) {
    symbol(i)
    [ and ]
    roman(i)
    [ for #i]
    parbreak()
}

---
// Error: 8-15 cannot convert integers greater than 3,999,999 to roman numerals
#roman(8000000)

---
// Error: 9-11 number must not be negative
#symbol(-1)
