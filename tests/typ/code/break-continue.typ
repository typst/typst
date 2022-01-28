// Test break and continue in loops.
// Ref: false

---
#for i in range(10) {
  if i > 5 {
    // Error: 5-10 break is not yet implemented
    break
  }
}

---
// Error: 1-10 unexpected keyword `continue`
#continue
