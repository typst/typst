// Test break and continue in loops.
// Ref: false

---
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

---
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

---
// Test continue.

#let i = 0
#let x = 0

#while x < 8 {
  i += 1
  if mod(i, 3) == 0 {
    continue
  }
  x += i
}

// If continue did not work, this would equal 10.
#test(x, 12)

---
// Test joining with continue.

#let x = for i in range(5) {
  "a"
  if mod(i, 3) == 0 {
    "_"
    continue
  }
  str(i)
}

#test(x, "a_a1a2a_a4")

---
// Test break outside of loop.

#let f() = {
  // Error: 3-8 cannot break outside of loop
  break
}

#f()

---
// Test continue outside of loop.

// Error: 12-20 cannot continue outside of loop
#let x = { continue }

---
// Error: 1-10 unexpected keyword `continue`
#continue
