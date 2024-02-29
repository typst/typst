// Issue #3154: Confusing errors from methods supposed to return a mutable entry
// https://github.com/typst/typst/issues/3154
// Ref: false

---
#{
  let array = ()
  // Error: 3-16 array is empty
  array.first()
}

---
#{
  let array = ()
  // Error: 3-16 array is empty
  array.first() = 9
}

---
#{
  let array = ()
  // Error: 3-15 array is empty
  array.last()
}

---
#{
  let array = ()
  // Error: 3-15 array is empty
  array.last() = 9
}

---
#{
  let array = (1,)
  // Error: 3-14 array index out of bounds (index: 1, len: 1) and no default value was specified
  array.at(1)
}

---
#{
  let array = (1,)
  test(array.at(1, default: 0), 0)
}

---
#{
  let array = (1,)
  // Error: 3-14 array index out of bounds (index: 1, len: 1)
  array.at(1) = 9
}

---
#{
  let array = (1,)
  // Error: 3-26 array index out of bounds (index: 1, len: 1)
  array.at(1, default: 0) = 9
}

---
#{
  let dict = (a: 1)
  // Error: 3-15 dictionary does not contain key "b" and no default value was specified
  dict.at("b")
}

---
#{
  let dict = (a: 1)
  test(dict.at("b", default: 0), 0)
}

---
#{
  let dict = (a: 1)
  // Error: 3-15 dictionary does not contain key "b"
  // Hint: 3-15 use `insert` to add or update values
  dict.at("b") = 9
}

---
#{
  let dict = (a: 1)
  // Error: 3-27 dictionary does not contain key "b"
  // Hint: 3-27 use `insert` to add or update values
  dict.at("b", default: 0) = 9
}

---
#{
  let dict = (a: 1)
  // Error: 8-9 dictionary does not contain key "b"
  dict.b
}

---
#{
  let dict = (a: 1)
  dict.b = 9
  test(dict, (a: 1, b: 9))
}

---
#{
  let dict = (a: 1)
  // Error: 3-9 dictionary does not contain key "b"
  // Hint: 3-9 use `insert` to add or update values
  dict.b += 9
}
