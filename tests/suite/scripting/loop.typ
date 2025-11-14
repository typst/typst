// Test break and continue in loops.

--- loop-break-basic paged ---
// Test break.

#let var = 0
#let error = false

#for i in range(10) {
  var += i
  if i > 5 {
    break
    error = true
  }
}

#test(var, 21)
#test(error, false)

--- loop-break-join-basic paged ---
// Test joining with break.

#let i = 0
#let x = while true {
  i += 1
  str(i)
  if i >= 5 {
    "."
    break
  }
}

#test(x, "12345.")

--- loop-continue-basic paged ---
// Test continue.

#let i = 0
#let x = 0

#while x < 8 {
  i += 1
  if calc.rem(i, 3) == 0 {
    continue
  }
  x += i
}

// If continue did not work, this would equal 10.
#test(x, 12)

--- loop-continue-join paged ---
// Test joining with continue.

#let x = for i in range(5) {
  "a"
  if calc.rem(i, 3) == 0 {
    "_"
    continue
  }
  str(i)
}

#test(x, "a_a1a2a_a4")

--- loop-break-outside-of-loop paged ---
// Test break outside of loop.
#let f() = {
  // Error: 3-8 cannot break outside of loop
  break
}

#for i in range(1) {
  f()
}

--- loop-break-join-in-last-arg paged ---
// Test break in function call.
#let identity(x) = x
#let out = for i in range(5) {
  "A"
  identity({
    "B"
    break
  })
  "C"
}

#test(out, "AB")

--- loop-continue-outside-of-loop-in-block paged ---
// Test continue outside of loop.

// Error: 12-20 cannot continue outside of loop
#let x = { continue }

--- loop-continue-outside-of-loop-in-markup paged ---
// Error: 2-10 cannot continue outside of loop
#continue

--- loop-break-join-in-nested-blocks paged ---
// Should output `Hello World 🌎`.
#for _ in range(10) {
  [Hello ]
  [World #{
    [🌎]
    break
  }]
}

--- loop-break-join-set-and-show paged ---
// Should output `Some` in red, `Some` in blue and `Last` in green.
// Everything should be in smallcaps.
#for color in (red, blue, green, yellow) [
  #set text(font: "Roboto")
  #show: it => text(fill: color, it)
  #smallcaps(if color != green [
    Some
  ] else [
    Last
    #break
  ])
]

--- loop-break-join-in-set-rule-args paged ---
// Test break in set rule.
// Should output `Hi` in blue.
#for i in range(10) {
  [Hello]
  set text(blue, ..break)
  [Not happening]
}

--- loop-break-join-in-first-arg paged ---
// Test second block during break flow.

#for i in range(10) {
  table(
    { [A]; break },
    for _ in range(3) [B]
  )
}
