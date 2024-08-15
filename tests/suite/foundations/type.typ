--- type ---
#test(type(1), int)
#test(type(ltr), direction)
#test(type(10 / 3), float)

--- type-string-compatibility ---
#test(type(10), int)
#test(type(10), "integer")
#test("is " + type(10), "is integer")
#test(int in ("integer", "string"), true)
#test(int in "integers or strings", true)
#test(str in "integers or strings", true)

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
