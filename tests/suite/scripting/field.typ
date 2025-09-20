// Test field access.

--- field-function ---
// Test fields on function scopes.
#enum.item
#assert.eq
#assert.ne

--- field-normal-function-invalid ---
// Error: 9-16 function `assert` does not contain field `invalid`
#assert.invalid

--- field-elem-function-invalid ---
// Error: 7-14 function `enum` does not contain field `invalid`
#enum.invalid

--- field-elem-function-invalid-call ---
// Error: 7-14 function `enum` does not contain field `invalid`
#enum.invalid()

--- field-closure-invalid ---
// Closures cannot have fields.
#let f(x) = x
// Error: 4-11 cannot access fields on user-defined functions
#f.invalid

--- field-bool-invalid ---
// Error: 8-10 cannot access fields on type boolean
#false.ok

--- field-bool-keyword-invalid ---
// Error: 9-13 cannot access fields on type boolean
#{false.true}

--- field-invalid-none ---
#{
  let object = none
  // Error: 3-9 none does not have accessible fields
  object.property = "value"
}

--- field-invalid-int ---
#{
  let object = 10
  // Error: 3-9 integer does not have accessible fields
  object.property = "value"
}

--- field-mutable-invalid-symbol ---
#{
  let object = sym.eq.not
  // Error: 3-9 cannot mutate fields on symbol
  object.property = "value"
}

--- field-mutable-invalid-module ---
#{
  let object = calc
  // Error: 3-9 cannot mutate fields on module
  object.property = "value"
}

--- field-mutable-invalid-function ---
#{
  let object = calc.sin
  // Error: 3-9 cannot mutate fields on function
  object.property = "value"
}

--- field-mutable-invalid-stroke ---
#{
  let s = 1pt + red
  // Error: 3-4 fields on stroke are not yet mutable
  // Hint: 3-4 try creating a new stroke with the updated field value instead
  s.thickness = 5pt
}

--- field-mutable-cannot-access ---
// Error: 8-12 cannot access mutating fields on array
#array.push
