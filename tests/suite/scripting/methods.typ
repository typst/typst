// Test method calls in code and in math.

// A pattern used in some of the `method-accent-*` tests is to create a symbol
// with an accent character (`sym.tilde`) as a field. This lets us create a
// callable method with an arbitrary name without the inconvenience of creating
// a whole module.

--- method-whitespace eval ---
// We allow whitespace around the dot.
#test( "Hi there" . split() , ("Hi", "there"))

--- method-multiline eval ---
// Test multiline chain in code block.
#{
  let rewritten = "Hello. This is a sentence. And one more."
    .split(".")
    .map(s => s.trim())
    .filter(s => s != "")
    .map(s => s + "!")
    .join("\n ")

  test(rewritten, "Hello!\n This is a sentence!\n And one more!")
}

--- math-method-accent-accent eval ---
// Test math method call for an accent symbol.
#test($arrow.l.r(x)$, $#math.arrow.l.r[\x]$)

--- math-method-accent-non-func eval ---
// Test calling a symbol whose field isn't an accent.
#test($pi.alt(x)$, $pi.alt/**/(x)$)

--- method-import-module eval ---
#import "module.typ"
#test(module.item(1, 2), 3)

--- math-method-import-module eval ---
#import "module.typ"
#test($module.item(#1, #2)$, $#3$)

--- method-missing-field eval ---
// Error: 6-13 module `pdf` does not contain `attache`
#pdf.attache()

--- math-method-missing-field eval ---
// Error: 14-19 function `assert` does not contain field `noteq`
$ std.assert.noteq() $

--- method-unknown-generic-type eval ---
// Error: 2-8 type array has no method `fun`
#().fun()

--- math-method-unknown-generic-type eval ---
// Error: 3-14 type color has no method `fun`
$ std.red.fun() $

--- method-unknown-content eval ---
// Error: 2-8 element sequence has no method `fun`
#[].fun()

--- math-method-unknown-content eval ---
#let pi = [pi]
// Error: 3-9 element text has no method `fun`
$ pi.fun() $

--- method-invalid-content-field-func eval ---
// This would be cursed to allow.
// Error: 2-48 `supplement` is not a valid method for element `heading`
#heading([], supplement: x => x + 1).supplement(2)

--- math-method-invalid-content-field-func eval ---
#let cancel = math.cancel($x$, angle: ang => ang + 90deg)
// Error: 3-15 `angle` is not a valid method for element `cancel`
$ cancel.angle(#30deg) $

--- method-invalid-non-func eval ---
// Error: 2-26 `stroke` is not a valid method for element `line`
// Hint: 2-26 this looks like a method call, but `line(stroke: red).stroke` produced type `stroke`
#line(stroke: red).stroke()

--- math-method-invalid-non-func eval ---
// Error: 27-36 `y` is not a valid method for type `alignment`
// Hint: 27-36 this looks like a method call, but `std.top.y` produced type `alignment`
// Hint: 27-36 try adding a space before the parentheses
#test($std.top.y/**/()$, $std.top.y()$)
// This is potentially worth allowing in the future.

--- method-dict-invalid eval ---
// Test attempting to call a function from a dictionary.
// Error: 2-34 cannot directly call dictionary keys as methods
// Hint: 2-34 dictionaries cannot use method syntax with keys, as keys could conflict with built-in method names
// Hint: 2-34 to call the function, wrap the field access in parentheses: `((call-me: () => "maybe").call-me)(..)`
#(call-me: () => "maybe").call-me()

--- math-method-dict-invalid eval ---
#let pi = (alt: _ => math.pi.alt)
// Error: 3-9 cannot directly call dictionary keys as methods
// Hint: 3-9 dictionaries cannot use method syntax with keys, as keys could conflict with built-in method names
// Hint: 3-9 to call the function, use code mode and wrap the field access in parentheses: `#(pi.alt)(..)`
$ pi.alt() $

--- method-dict-non-func eval ---
// Error: 2-24 cannot directly call dictionary keys as methods
// Hint: 2-24 dictionaries cannot use method syntax with keys, as keys could conflict with built-in method names
// Hint: 2-24 this looks like a method call, but `(non-func: 1).non-func` produced type `integer`
#(non-func: 1).non-func()

--- math-method-dict-non-func eval ---
// The hint should differ slightly to account for being in math mode.
#let pi = (alt: math.pi.alt)
// Error: 3-9 cannot directly call dictionary keys as methods
// Hint: 3-9 dictionaries cannot use method syntax with keys, as keys could conflict with built-in method names
// Hint: 3-9 this looks like a method call, but `pi.alt` produced type `symbol`
// Hint: 3-9 try adding a space before the parentheses
$ pi.alt() $

--- method-mut-basic eval ---
// Mutating methods mutate a variable.
#let numbers = (1, 2, 3)
#test(numbers.remove(1), 2)
#test(numbers, (1, 3))

--- math-method-mut-basic eval ---
// Test mutating method calls in math.
// Currently they just error, but may be allowed in the future.
#let array = (1, 2)
#let dict = (one: 1)
#let _ = $
  // Error: 3-8 cannot mutate a temporary value
  array.push("two")
  array.insert(#2, dict.remove("one"))
  dict.insert(array.pop(), array.remove(#1))
$
#test(array, (1, 1))
#test(dict, (two: 2))

--- method-mut-temporary eval ---
#let numbers = (1, 2, 3)
// Error: 2-43 cannot mutate a temporary value
#numbers.map(v => v / 2).sorted().map(str).remove(4)

--- method-mut-constant eval ---
// Error: 2-5 cannot mutate a constant: box
#box.push(1)

--- method-dict-pop eval ---
// `pop` is a mutating method on arrays, but not dictionaries.
#let dict = (pop: () => none)
// Error: 2-10 cannot directly call dictionary keys as methods
// Hint: 2-10 dictionaries cannot use method syntax with keys, as keys could conflict with built-in method names
// Hint: 2-10 to call the function, wrap the field access in parentheses: `(dict.pop)(..)`
#dict.pop()

--- method-dict-non-func-pop eval ---
#let dict = (pop: none)
// Error: 2-10 cannot directly call dictionary keys as methods
// Hint: 2-10 dictionaries cannot use method syntax with keys, as keys could conflict with built-in method names
// Hint: 2-10 this looks like a method call, but `dict.pop` produced type `none`
#dict.pop()

--- math-method-dict-non-func-pop eval ---
#let dict = (pop: none)
// Error: 3-7 cannot mutate a temporary value
$ dict.pop() $

--- method-dict-non-func-pop-arg eval ---
#let dict = (pop: none)
// Error: 2-10 cannot directly call dictionary keys as methods
// Hint: 2-10 dictionaries cannot use method syntax with keys, as keys could conflict with built-in method names
// Hint: 2-10 this looks like a method call, but `dict.pop` produced type `none`
#dict.pop(arg: "something")

--- math-method-dict-non-func-pop-arg eval ---
#let dict = (pop: none)
// Error: 3-7 cannot mutate a temporary value
$ dict.pop(arg: "something") $

--- method-mut-access eval ---
// Test calling a mutating method from accessor methods.
#{
  let matrix = (((1,), (2,)), ((3,), (4,)))
  matrix.at(1).at(0).push(5)
  test(matrix, (((1,), (2,)), ((3, 5), (4,))))
}

--- method-mut-access-module-push eval ---
// Edge case for module access that isn't fixed.
#import "module.typ"
// Works because the method name isn't categorized as mutating.
#test((module,).at(0).item(1, 2), 3)
// Doesn't work because of mutating name.
// Error: 7-16 cannot mutate a temporary value
#test((module,).at(0).push(2), 3)

--- math-method-mut-access-module-push eval ---
// Edge case for module access that isn't fixed.
#import "module.typ"
#let indirect = (mod: module)
// Doesn't work because of mutating method name.
// Error: 8-16 cannot mutate a temporary value
#test($indirect.mod.push(#2)$, $#3$)

--- method-mut-eval-order eval ---
// Test evaluation order of mutating methods with accessors.
#{
  let pair = (1, 2)
  let arrays = ((), (), ())
  arrays.at(pair.remove(0)).push(pair.remove(0))
  //             ^^^^^^ second (2)    ^^^^^^ first (1)
  test(arrays, ((), (), (1,)))
}

--- method-mut-eval-order-self eval ---
// Test evaluation order when arguments mutate the variable itself.
#{
  let pair = ((0,), (1,))
  // Error: 3-13 array index out of bounds (index: 1, len: 1)
  pair.at(1).insert(0, pair.remove(0))
}

--- method-mut-eval-order-assign eval ---
// Test evaluation order when assigning to a variable in an argument.
#{
  let what = ()
  what.insert("what", what = (:))
  test(what, (what: none))
}

--- method-mut-eval-order-replace eval ---
// Test whether replacing a field while accessing it causes an error.
#{
  let dict = (one: ())
  dict.one.insert("two", dict.insert("one", (:)))
  test(dict.one, (two: none))
}

--- method-mut-eval-order-replace-nested eval ---
// Test whether replacing a nested field while accessing it causes an error.
#{
  let dict = (one: (two: ()))
  dict.one.two.insert("three", dict.insert("one", (two: (:))))
  test(dict.one, (two: (three: none)))
}

--- method-accent-eval-order-assign eval ---
// Test evaluation order when assigning to a variable in an argument.
#{
  let sm = symbol("m", ("method", sym.tilde))
  let array = ()
  // Error: 8-20 type array has no method `method`
  test(array.method(array = sm), sym.tilde(none))
  test(array, sm)
}

--- math-method-accent-eval-order-assign eval ---
// Test evaluation order when assigning to a variable in an argument in math.
#{
  let sm = symbol("m", ("method", sym.tilde))
  let array = ()
  // Error: 9-21 type array has no method `method`
  test($array.method(#{array = sm})$, $#sym.tilde(none)$)
  test(array, sm)
}

--- method-accent-eval-order-shadowed eval ---
// Test shadowing a variable in arguments while calling a method on it.
#{
  let sm = symbol("m", ("method", sym.tilde))
  test(sm.method(let sm = false), sym.tilde(none))
  test(sm, false)
}

--- math-method-accent-eval-order-shadowed eval ---
// Test shadowing a variable in arguments while calling a method on it in math.
#{
  let sm = symbol("m", ("method", sym.tilde))
  test($sm.method(#let sm = false;)$, $#sym.tilde(none)$)
  test(sm, false)
}

--- method-accent-eval-order-shadowed-push eval ---
// This differs because `push` is a mutating method name, even though the method
// on `sp` here isn't actually mutating.
#{
  let sp = symbol("p", ("push", sym.tilde))
  // Error: 8-15 type boolean has no method `push`
  test(sp.push(let sp = false), sym.tilde(none))
  test(sp, false)
}

--- math-method-accent-eval-order-shadowed-push eval ---
// This just errors like `math-method-mut`.
#{
  let sp = symbol("p", ("push", sym.tilde))
  // Error: 9-11 cannot mutate a temporary value
  test($sp.push(#let sp = false;)$, $#sym.tilde(none)$)
  test(sp, false)
}

--- method-accent-indirect-non-mut eval ---
// Using a non-mutating method, `dict.sym.push()`, in its own argument, but
// indirectly via a mutating method, `sym-sym.pop()`.
#{
  let sp-dict = (sym: symbol("p", ("push", sym.tilde)))
  let array = ("sym", "sym")
  let result = sp-dict
    .at(array.pop())
    .push(
      sp-dict.at(array.pop()).push(none),
    )
  test(result, sym.tilde(sym.tilde(none)))
  test(array, ())
}

--- method-accent-assign-during-non-mut-access eval ---
// Using a non-mutating method, `dict.sym.push()`, in an assignment, but
// indirectly via a mutating method, `sym-sym.pop()`.
#{
  let sp-dict = (sym: symbol("p", ("push", sym.tilde)))
  let array = ("sym", "sym")
  sp-dict.at(array.pop()) = sp-dict.at(array.pop()).push(none)
  test(array, ())
}
