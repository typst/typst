// Test break and continue in loops.
// Ref: false

---
// Test break.

#let error = false
#let var = 0

#for i in range(10) {
  var += i
  if i > 5 {
    break
    error = true
  }
}

#test(error, false)
#test(var, 21)

---
// Test continue.

#let x = 0
#let i = 0

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
