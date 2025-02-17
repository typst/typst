--- type ---
#test(type(1), int)
#test(type(ltr), direction)
#test(type(10 / 3), float)
#test(type(10) == int, true)
#test(type(10) != int, false)

--- type-string-compatibility-add ---
// Warning: 7-23 adding strings and types is deprecated
// Hint: 7-23 convert the type to a string with `str` first
#test("is " + type(10), "is integer")
// Warning: 7-23 adding strings and types is deprecated
// Hint: 7-23 convert the type to a string with `str` first
#test(type(10) + " is", "integer is")

--- type-string-compatibility-join ---
// Warning: 16-24 joining strings and types is deprecated
// Hint: 16-24 convert the type to a string with `str` first
#test({ "is "; type(10) }, "is integer")
// Warning: 19-24 joining strings and types is deprecated
// Hint: 19-24 convert the type to a string with `str` first
#test({ type(10); " is" }, "integer is")

--- type-string-compatibility-equal ---
// Warning: 7-28 comparing strings with types is deprecated
// Hint: 7-28 compare with the literal type instead
// Hint: 7-28 this comparison will always return `false` in future Typst releases
#test(type(10) == "integer", true)
// Warning: 7-26 comparing strings with types is deprecated
// Hint: 7-26 compare with the literal type instead
// Hint: 7-26 this comparison will always return `false` in future Typst releases
#test(type(10) != "float", true)

--- type-string-compatibility-in-array ---
// Warning: 7-35 comparing strings with types is deprecated
// Hint: 7-35 compare with the literal type instead
// Hint: 7-35 this comparison will always return `false` in future Typst releases
#test(int in ("integer", "string"), true)
// Warning: 7-37 comparing strings with types is deprecated
// Hint: 7-37 compare with the literal type instead
// Hint: 7-37 this comparison will always return `false` in future Typst releases
#test(float in ("integer", "string"), false)

--- type-string-compatibility-in-str ---
// Warning: 7-35 checking whether a type is contained in a string is deprecated
// Hint: 7-35 this compatibility behavior only exists because `type` used to return a string
#test(int in "integers or strings", true)
// Warning: 7-35 checking whether a type is contained in a string is deprecated
// Hint: 7-35 this compatibility behavior only exists because `type` used to return a string
#test(str in "integers or strings", true)
// Warning: 7-37 checking whether a type is contained in a string is deprecated
// Hint: 7-37 this compatibility behavior only exists because `type` used to return a string
#test(float in "integers or strings", false)

--- type-string-compatibility-in-dict ---
// Warning: 7-37 checking whether a type is contained in a dictionary is deprecated
// Hint: 7-37 this compatibility behavior only exists because `type` used to return a string
#test(int in (integer: 1, string: 2), true)

--- issue-3110-type-constructor ---
// Let the error message report the type name.
// Error: 2-9 type content does not have a constructor
#content()

--- issue-3110-associated-field ---
// Error: 6-12 type integer does not contain field `MAXVAL`
#int.MAXVAL

--- issue-3110-associated-function ---
// Error: 6-18 type string does not contain field `from-unïcode`
#str.from-unïcode(97)

--- issue-2747-repr-auto-none ---
#test(repr(none), "none")
#test(repr(auto), "auto")
#test(repr(type(none)), "type(none)")
#test(repr(type(auto)), "type(auto)")
