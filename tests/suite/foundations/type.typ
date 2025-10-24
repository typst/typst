--- type paged ---
#test(type(1), int)
#test(type(ltr), direction)
#test(type(10 / 3), float)

--- issue-3110-type-constructor paged ---
// Let the error message report the type name.
// Error: 2-9 type content does not have a constructor
#content()

--- issue-3110-associated-field paged ---
// Error: 6-12 type integer does not contain field `MAXVAL`
#int.MAXVAL

--- issue-3110-associated-function paged ---
// Error: 6-18 type string does not contain field `from-unïcode`
#str.from-unïcode(97)

--- issue-2747-repr-auto-none paged ---
#test(repr(none), "none")
#test(repr(auto), "auto")
#test(repr(type(none)), "type(none)")
#test(repr(type(auto)), "type(auto)")
