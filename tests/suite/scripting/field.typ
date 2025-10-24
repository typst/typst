// Test field access.

--- field-function render ---
// Test fields on function scopes.
#enum.item
#assert.eq
#assert.ne

--- field-normal-function-invalid render ---
// Error: 9-16 function `assert` does not contain field `invalid`
#assert.invalid

--- field-elem-function-invalid render ---
// Error: 7-14 function `enum` does not contain field `invalid`
#enum.invalid

--- field-elem-function-invalid-call render ---
// Error: 7-14 function `enum` does not contain field `invalid`
#enum.invalid()

--- field-closure-invalid render ---
// Closures cannot have fields.
#let f(x) = x
// Error: 4-11 cannot access fields on user-defined functions
#f.invalid

--- field-bool-invalid render ---
// Error: 8-10 cannot access fields on type boolean
#false.ok

--- field-bool-keyword-invalid render ---
// Error: 9-13 cannot access fields on type boolean
#{false.true}

--- field-invalid-none render ---
#{
  let object = none
  // Error: 3-9 none does not have accessible fields
  object.property = "value"
}

--- field-invalid-int render ---
#{
  let object = 10
  // Error: 3-9 integer does not have accessible fields
  object.property = "value"
}

--- field-mutable-invalid-symbol render ---
#{
  let object = sym.eq.not
  // Error: 3-9 cannot mutate fields on symbol
  object.property = "value"
}

--- field-mutable-invalid-module render ---
#{
  let object = calc
  // Error: 3-9 cannot mutate fields on module
  object.property = "value"
}

--- field-mutable-invalid-function render ---
#{
  let object = calc.sin
  // Error: 3-9 cannot mutate fields on function
  object.property = "value"
}

--- field-mutable-invalid-stroke render ---
#{
  let s = 1pt + red
  // Error: 3-4 fields on stroke are not yet mutable
  // Hint: 3-4 try creating a new stroke with the updated field value instead
  s.thickness = 5pt
}
