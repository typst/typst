// In this bug, the hint and error messages for an equation
// being reference mentioned that it was a "heading" and was
// lacking the proper path.
// Ref: false

---
#set page(height: 70pt)

$
    Delta = b^2 - 4 a c
$ <quadratic>

// Error: 14-24 cannot reference equation without numbering
// Hint: 14-24 you can enable equation numbering with `#set math.equation(numbering: "1.")`
Looks at the @quadratic formula.