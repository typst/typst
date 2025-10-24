// Test field access.

--- field-function render ---
// Test fields on function scopes.
#enum.item
#assert.eq
#assert.ne

--- field-normal-function-invalid paged diagnostic ---
// Error: 9-16 function `assert` does not contain field `invalid`
#assert.invalid

--- field-elem-function-invalid paged diagnostic ---
// Error: 7-14 function `enum` does not contain field `invalid`
#enum.invalid

--- field-elem-function-invalid-call paged diagnostic ---
// Error: 7-14 function `enum` does not contain field `invalid`
#enum.invalid()

--- field-closure-invalid render ---
// Closures cannot have fields.
#let f(x) = x
// Error: 4-11 cannot access fields on user-defined functions
#f.invalid

--- field-bool-invalid paged diagnostic ---
// Error: 8-10 cannot access fields on type boolean
#false.ok

--- field-bool-keyword-invalid paged diagnostic ---
// Error: 9-13 cannot access fields on type boolean
#{false.true}

--- field-invalid-none paged diagnostic ---
#{
  let object = none
  // Error: 3-9 none does not have accessible fields
  object.property = "value"
}

--- field-invalid-int paged diagnostic ---
#{
  let object = 10
  // Error: 3-9 integer does not have accessible fields
  object.property = "value"
}

--- field-mutable-invalid-symbol paged diagnostic ---
#{
  let object = sym.eq.not
  // Error: 3-9 cannot mutate fields on symbol
  object.property = "value"
}

--- field-mutable-invalid-module paged diagnostic ---
#{
  let object = calc
  // Error: 3-9 cannot mutate fields on module
  object.property = "value"
}

--- field-mutable-invalid-function paged diagnostic ---
#{
  let object = calc.sin
  // Error: 3-9 cannot mutate fields on function
  object.property = "value"
}

--- field-mutable-invalid-stroke paged diagnostic ---
#{
  let s = 1pt + red
  // Error: 3-4 fields on stroke are not yet mutable
  // Hint: 3-4 try creating a new stroke with the updated field value instead
  s.thickness = 5pt
}
