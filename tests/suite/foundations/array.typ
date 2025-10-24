// Test arrays.

--- array-basic-syntax render ---
#set page(width: 150pt)

// Empty.
#()

// Not an array, just a parenthesized expression.
#(1)

// One item and trailing comma.
#(-1,)

// No trailing comma.
#(true, false)

// Multiple lines and items and trailing comma.
#("1"
    , rgb("002")
    ,)

--- array-bad-token render ---
// Error: 4-6 unexpected end of block comment
// Hint: 4-6 consider escaping the `*` with a backslash or opening the block comment with `/*`
#(1*/2)

--- array-bad-number-suffix render ---
// Error: 6-8 invalid number suffix: u
#(1, 1u 2)

--- array-leading-comma render ---
// Error: 3-4 unexpected comma
#(,1)

--- array-incomplete-pair render ---
// Missing expression makes named pair incomplete, making this an empty array.
// Error: 5 expected expression
#(a:)

--- array-named-pair render ---
// Named pair after this is already identified as an array.
// Error: 6-10 expected expression, found named pair
#(1, b: 2)

--- array-keyed-pair render ---
// Keyed pair after this is already identified as an array.
// Error: 6-14 expected expression, found keyed pair
#(1, "key": 2)

--- array-bad-conversion-from-string render ---
// Error: 8-15 expected array, bytes, or version, found string
#array("hello")

--- spread-into-array render ---
// Test spreading into array and dictionary.
#{
  let l = (1, 2, 3)
  let r = (5, 6, 7)
  test((..l, 4, ..r), range(1, 8))
  test((..none), ())
}

--- spread-dict-into-array render ---
// Error: 9-17 cannot spread dictionary into array
#(1, 2, ..(a: 1))

--- array-len render ---
// Test the `len` method.
#test(().len(), 0)
#test(("A", "B", "C").len(), 3)

--- array-at-lvalue render ---
// Test lvalue and rvalue access.
#{
  let array = (1, 2)
  array.at(1) += 5 + array.at(0)
  test(array, (1, 8))
}

--- array-first-and-at-lvalue render ---
// Test different lvalue method.
#{
  let array = (1, 2, 3)
  array.first() = 7
  array.at(1) *= 8
  test(array, (7, 16, 3))
}

--- array-at-out-of-bounds render ---
// Test rvalue out of bounds.
// Error: 2-17 array index out of bounds (index: 5, len: 3) and no default value was specified
#(1, 2, 3).at(5)

--- array-at-out-of-bounds-negative render ---
// Error: 2-18 array index out of bounds (index: -4, len: 3) and no default value was specified
#(1, 2, 3).at(-4)

--- array-at-out-of-bounds-lvalue render ---
// Test lvalue out of bounds.
#{
  let array = (1, 2, 3)
  // Error: 3-14 array index out of bounds (index: 3, len: 3)
  array.at(3) = 5
}

--- array-at-with-default render ---
// Test default value.
#test((1, 2, 3).at(2, default: 5), 3)
#test((1, 2, 3).at(3, default: 5), 5)

--- array-remove-with-default render ---
// Test remove with default value.

#{
  let array = (1, 2, 3)
  test(array.remove(2, default: 5), 3)
}

#{
  let array = (1, 2, 3)
  test(array.remove(3, default: 5), 5)
}

--- array-range render ---
// Test the `range` function.
#test(range(4), (0, 1, 2, 3))
#test(range(1, 4), (1, 2, 3))
#test(range(-4, 2), (-4, -3, -2, -1, 0, 1))
#test(range(10, 5), ())
#test(range(10, step: 3), (0, 3, 6, 9))
#test(range(1, 4, step: 1), (1, 2, 3))
#test(range(1, 8, step: 2), (1, 3, 5, 7))
#test(range(5, 2, step: -1), (5, 4, 3))
#test(range(10, 0, step: -3), (10, 7, 4, 1))

--- array-range-end-missing render ---
// Error: 2-9 missing argument: end
#range()

--- array-range-float-invalid render ---
// Error: 11-14 expected integer, found float
#range(1, 2.0)

--- array-range-bad-step-type render ---
// Error: 17-22 expected integer, found string
#range(4, step: "one")

--- array-range-step-zero render ---
// Error: 18-19 number must not be zero
#range(10, step: 0)

--- array-bad-method-lvalue render ---
// Test bad lvalue.
// Error: 2:3-2:14 cannot mutate a temporary value
#let array = (1, 2, 3)
#(array.len() = 4)

--- array-unknown-method-lvalue render ---
// Test bad lvalue.
// Error: 2:9-2:13 type array has no method `yolo`
#let array = (1, 2, 3)
#(array.yolo() = 4)

--- array-negative-indices render ---
// Test negative indices.
#{
  let array = (1, 2, 3, 4)
  test(array.at(0), 1)
  test(array.at(-1), 4)
  test(array.at(-2), 3)
  test(array.at(-3), 2)
  test(array.at(-4), 1)
}

--- array-first-and-last render ---
// The `first` and `last` methods.
#test((1,).first(), 1)
#test((2,).last(), 2)
#test((1, 2, 3).first(), 1)
#test((1, 2, 3).last(), 3)
#test((1, 2).first(default: 99), 1)
#test(().first(default: 99), 99)
#test((1, 2).last(default: 99), 2)
#test(().last(default: 99), 99)

--- array-first-empty render ---
// Error: 2-12 array is empty
#().first()

--- array-last-empty render ---
// Error: 2-11 array is empty
#().last()

--- array-push-and-pop render ---
// Test the `push` and `pop` methods.
#{
  let tasks = (a: (1, 2, 3), b: (4, 5, 6))
  test(tasks.at("a").pop(), 3)
  tasks.b.push(7)
  test(tasks.a, (1, 2))
  test(tasks.at("b"), (4, 5, 6, 7))
}

--- array-insert-and-remove render ---
// Test the `insert` and `remove` methods.
#{
  let array = (0, 1, 2, 4, 5)
  array.insert(3, 3)
  test(array, range(6))
  array.remove(1)
  test(array, (0, 2, 3, 4, 5))
}

--- array-insert-missing-index render ---
// Error: 2:2-2:18 missing argument: index
#let numbers = ()
#numbers.insert()

--- array-slice render ---
// Test the `slice` method.
#test((1, 2, 3, 4).slice(2), (3, 4))
#test(range(10).slice(2, 6), (2, 3, 4, 5))
#test(range(10).slice(4, count: 3), (4, 5, 6))
#test(range(10).slice(-5, count: 2), (5, 6))
#test((1, 2, 3).slice(-3, count: 3), (1, 2, 3))
#test((1, 2, 3).slice(-1, count: 1), (3,))
#test((1, 2, 3).slice(2, -2), ())
#test((1, 2, 3).slice(-2, 2), (2,))
#test((1, 2, 3).slice(-3, 2), (1, 2))
#test("ABCD".split("").slice(1, -1).join("-"), "A-B-C-D")

--- array-slice-out-of-bounds render ---
// Error: 2-30 array index out of bounds (index: 12, len: 10)
#range(10).slice(9, count: 3)

--- array-slice-out-of-bounds-from-back render ---
// Error: 2-31 array index out of bounds (index: 12, len: 10)
#range(10).slice(-2, count: 4)

--- array-slice-out-of-bounds-negative render ---
// Error: 2-24 array index out of bounds (index: -4, len: 3)
#(1, 2, 3).slice(0, -4)

--- array-position render ---
// Test the `position` method.
#test(("Hi", "❤️", "Love").position(s => s == "❤️"), 1)
#test(("Bye", "💘", "Apart").position(s => s == "❤️"), none)
#test(("A", "B", "CDEF", "G").position(v => v.len() > 2), 2)

--- array-filter render ---
// Test the `filter` method.
#test(().filter(calc.even), ())
#test((1, 2, 3, 4).filter(calc.even), (2, 4))
#test((7, 3, 2, 5, 1).filter(x => x < 5), (3, 2, 1))

--- array-map render ---
// Test the `map` method.
#test(().map(x => x * 2), ())
#test((2, 3).map(x => x * 2), (4, 6))

--- array-fold render ---
// Test the `fold` method.
#test(().fold("hi", grid), "hi")
#test((1, 2, 3, 4).fold(0, (s, x) => s + x), 10)

--- array-fold-closure-without-params render ---
// Error: 20-22 unexpected argument
#(1, 2, 3).fold(0, () => none)

--- array-sum render ---
// Test the `sum` method.
#test(().sum(default: 0), 0)
#test(().sum(default: []), [])
#test((1, 2, 3).sum(), 6)

--- array-sum-empty render ---
// Error: 2-10 cannot calculate sum of empty array with no default
#().sum()

--- array-product render ---
// Test the `product` method.
#test(().product(default: 0), 0)
#test(().product(default: []), [])
#test(([ab], 3).product(), [ab]*3)
#test((1, 2, 3).product(), 6)

--- array-product-empty render ---
// Error: 2-14 cannot calculate product of empty array with no default
#().product()

--- array-rev render ---
// Test the `rev` method.
#test(range(3).rev(), (2, 1, 0))

--- array-join render ---
// Test the `join` method.
#test(().join(), none)
#test((1,).join(), 1)
#test(("a", "b", "c").join(), "abc")
#test("(" + ("a", "b", "c").join(", ") + ")", "(a, b, c)")

--- array-join-default render ---
#test(().join(default: "EMPTY", ", "), "EMPTY")
#test(("hello",).join(default: "EMPTY", ", "), "hello")
#test(("hello", "world").join(default: "EMPTY", ", "), "hello, world")

--- array-join-bad-values render ---
// Error: 2-22 cannot join boolean with boolean
#(true, false).join()

--- array-join-bad-separator render ---
// Error: 2-20 cannot join string with integer
#("a", "b").join(1)

--- array-join-content render ---
// Test joining content.
#([One], [Two], [Three]).join([, ], last: [ and ]).

--- array-intersperse render ---
// Test the `intersperse` method
#test(().intersperse("a"), ())
#test((1,).intersperse("a"), (1,))
#test((1, 2).intersperse("a"), (1, "a", 2))
#test((1, 2, "b").intersperse("a"), (1, "a", 2, "a", "b"))

--- array-chunks render ---
// Test the `chunks` method.
#test(().chunks(10), ())
#test((1, 2, 3).chunks(10), ((1, 2, 3),))
#test((1, 2, 3, 4, 5, 6).chunks(3), ((1, 2, 3), (4, 5, 6)))
#test((1, 2, 3, 4, 5, 6, 7, 8).chunks(3), ((1, 2, 3), (4, 5, 6), (7, 8)))

#test(().chunks(10, exact: true), ())
#test((1, 2, 3).chunks(10, exact: true), ())
#test((1, 2, 3, 4, 5, 6).chunks(3, exact: true), ((1, 2, 3), (4, 5, 6)))
#test((1, 2, 3, 4, 5, 6, 7, 8).chunks(3, exact: true), ((1, 2, 3), (4, 5, 6)))

--- array-chunks-size-zero render ---
// Error: 19-20 number must be positive
#(1, 2, 3).chunks(0)

--- array-chunks-size-negative render ---
// Error: 19-21 number must be positive
#(1, 2, 3).chunks(-5)

--- array-windows render ---
// Test the `windows` method.
#test(().windows(5), ())
#test((1, 2, 3).windows(5), ())
#test((1, 2, 3, 4, 5).windows(3), ((1, 2, 3), (2, 3, 4), (3, 4, 5)))
#test((1, 2, 3, 4, 5, 6, 7, 8).windows(5), ((1, 2, 3, 4, 5), (2, 3, 4, 5, 6), (3, 4, 5, 6, 7), (4, 5, 6, 7, 8)))

--- array-windows-size-zero render ---
// Error: 20-21 number must be positive
#(1, 2, 3).windows(0)

--- array-windows-size-negative render ---
// Error: 20-22 number must be positive
#(1, 2, 3).windows(-5)

--- array-sorted render ---
// Test the `sorted` method.
#test(().sorted(), ())
#test(().sorted(key: x => x), ())
#test(((true, false) * 10).sorted(), (false,) * 10 + (true,) * 10)
#test(("it", "the", "hi", "text").sorted(), ("hi", "it", "text", "the"))
#test(("I", "the", "hi", "text").sorted(key: x => x), ("I", "hi", "text", "the"))
#test(("I", "the", "hi", "text").sorted(key: x => x.len()), ("I", "hi", "the", "text"))
#test((2, 1, 3, 10, 5, 8, 6, -7, 2).sorted(), (-7, 1, 2, 2, 3, 5, 6, 8, 10))
#test((2, 1, 3, -10, -5, 8, 6, -7, 2).sorted(key: x => x), (-10, -7, -5, 1, 2, 2, 3, 6, 8))
#test((2, 1, 3, -10, -5, 8, 6, -7, 2).sorted(key: x => x * x), (1, 2, 2, 3, -5, 6, -7, 8, -10))
#test(("I", "the", "hi", "text").sorted(by: (x, y) => x.len() < y.len()), ("I", "hi", "the", "text"))
#test(("I", "the", "hi", "text").sorted(key: x => x.len(), by: (x, y) => y < x), ("text", "the", "hi", "I"))

--- array-sorted-invalid-by-function render ---
// Error: 2-39 expected boolean from `by` function, got string
#(1, 2, 3).sorted(by: (_, _) => "hmm")

--- array-sorted-key-function-positional-1 render ---
// Error: 12-18 unexpected argument
#().sorted(x => x)

--- array-zip render ---
// Test the `zip` method.
#test(().zip(()), ())
#test((1,).zip(()), ())
#test((1,).zip((2,)), ((1, 2),))
#test((1, 2).zip((3, 4)), ((1, 3), (2, 4)))
#test((1, 2).zip((3, 4), exact: true), ((1, 3), (2, 4)))
#test((1, 2, 3, 4).zip((5, 6)), ((1, 5), (2, 6)))
#test(((1, 2), 3).zip((4, 5)), (((1, 2), 4), (3, 5)))
#test((1, "hi").zip((true, false)), ((1, true), ("hi", false)))
#test((1, 2, 3).zip((3, 4, 5), (6, 7, 8)), ((1, 3, 6), (2, 4, 7), (3, 5, 8)))
#test(().zip((), ()), ())
#test((1,).zip((2,), (3,)), ((1, 2, 3),))
#test((1, 2, 3).zip(), ((1,), (2,), (3,)))
#test(array.zip(()), ())

--- array-zip-exact-error render ---
// Error: 13-22 second array has different length (3) from first array (2)
#(1, 2).zip((1, 2, 3), exact: true)

--- array-zip-exact-multi-error render ---
// Error: 13-22 array has different length (3) from first array (2)
// Error: 24-36 array has different length (4) from first array (2)
#(1, 2).zip((1, 2, 3), (1, 2, 3, 4), exact: true)

--- array-enumerate render ---
// Test the `enumerate` method.
#test(().enumerate(), ())
#test(().enumerate(start: 5), ())
#test(("a", "b", "c").enumerate(), ((0, "a"), (1, "b"), (2, "c")))
#test(("a", "b", "c").enumerate(start: 1), ((1, "a"), (2, "b"), (3, "c")))
#test(("a", "b", "c").enumerate(start: 42), ((42, "a"), (43, "b"), (44, "c")))
#test(("a", "b", "c").enumerate(start: -7), ((-7, "a"), (-6, "b"), (-5, "c")))

--- array-dedup render ---
// Test the `dedup` method.
#test(().dedup(), ())
#test((1,).dedup(), (1,))
#test((1, 1).dedup(), (1,))
#test((1, 2, 1).dedup(), (1, 2))
#test(("Jane", "John", "Eric").dedup(), ("Jane", "John", "Eric"))
#test(("Jane", "John", "Eric", "John").dedup(), ("Jane", "John", "Eric"))

--- array-dedup-key render ---
// Test the `dedup` method with the `key` argument.
#test((1, 2, 3, 4, 5, 6).dedup(key: x => calc.rem(x, 2)), (1, 2))
#test((1, 2, 3, 4, 5, 6).dedup(key: x => calc.rem(x, 3)), (1, 2, 3))
#test(("Hello", "World", "Hi", "There").dedup(key: x => x.len()), ("Hello", "Hi"))
#test(("Hello", "World", "Hi", "There").dedup(key: x => x.at(0)), ("Hello", "World", "There"))

--- array-to-dict render ---
// Test the `to-dict` method.
#test(().to-dict(), (:))
#test((("a", 1), ("b", 2), ("c", 3)).to-dict(), (a: 1, b: 2, c: 3))
#test((("a", 1), ("b", 2), ("c", 3), ("b", 4)).to-dict(), (a: 1, b: 4, c: 3))

--- array-to-dict-bad-item-type render ---
// Error: 2-16 expected (str, any) pairs, found integer
#(1,).to-dict()

--- array-to-dict-bad-pair-length-1 render ---
// Error: 2-19 expected pairs of length 2, found length 1
#((1,),).to-dict()

--- array-to-dict-bad-pair-length-3 render ---
// Error: 2-26 expected pairs of length 2, found length 3
#(("key",1,2),).to-dict()

--- array-to-dict-bad-key-type render ---
// Error: 2-21 expected key of type str, found integer
#((1, 2),).to-dict()

--- array-zip-positional-and-named-argument render ---
// Error: 13-30 unexpected argument: val
#().zip((), val: "applicable")

--- array-sorted-bad-key render ---
// Error: 32-37 cannot divide by zero
#(1, 2, 0, 3).sorted(key: x => 5 / x)

--- array-sorted-uncomparable render ---
// Error: 2-26 cannot compare content and content
#([Hi], [There]).sorted()

--- array-sorted-uncomparable-lengths render ---
// Error: 2-26 cannot compare 3em with 2pt
#(1pt, 2pt, 3em).sorted()

--- array-sorted-key-function-positional-2 render ---
// Error: 42-52 unexpected argument
#((k: "a", v: 2), (k: "b", v: 1)).sorted(it => it.v)

--- issue-3014-mix-array-dictionary render ---
// Error: 8-17 expected expression, found named pair
#(box, fill: red)

--- issue-3154-array-first-empty render ---
#{
  let array = ()
  // Error: 3-16 array is empty
  array.first()
}

--- issue-3154-array-first-mutable-empty render ---
#{
  let array = ()
  // Error: 3-16 array is empty
  array.first() = 9
}

--- issue-3154-array-last-empty render ---
#{
  let array = ()
  // Error: 3-15 array is empty
  array.last()
}

--- issue-3154-array-last-mutable-empty render ---
#{
  let array = ()
  // Error: 3-15 array is empty
  array.last() = 9
}

--- issue-3154-array-at-out-of-bounds render ---
#{
  let array = (1,)
  // Error: 3-14 array index out of bounds (index: 1, len: 1) and no default value was specified
  array.at(1)
}

--- issue-3154-array-at-out-of-bounds-default render ---
#{
  let array = (1,)
  test(array.at(1, default: 0), 0)
}

--- issue-3154-array-at-out-of-bounds-mutable render ---
#{
  let array = (1,)
  // Error: 3-14 array index out of bounds (index: 1, len: 1)
  array.at(1) = 9
}

--- issue-3154-array-at-out-of-bounds-mutable-default render ---
#{
  let array = (1,)
  // Error: 3-26 array index out of bounds (index: 1, len: 1)
  array.at(1, default: 0) = 9
}

--- array-unopened render ---
// Error: 2-3 unclosed delimiter
#{)}

--- array-unclosed render ---
// Error: 3-4 unclosed delimiter
#{(}

--- array-reduce render ---
// Test the `reduce` method.
#test(().reduce(grid), none)
#test((1, 2, 3, 4).reduce((s, x) => s + x), 10)

--- array-reduce-missing-reducer render ---
// Error: 2-13 missing argument: reducer
#().reduce()

--- array-reduce-unexpected-argument render ---
// Error: 19-21 unexpected argument
#(1, 2, 3).reduce(() => none)
