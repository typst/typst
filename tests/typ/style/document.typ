// Test document and page-level styles.

---
// This is okay.
// Ref: false
#set document(title: "Hello")

---
Hello

// Error: 1-30 must appear before any content
#set document(title: "Hello")

---
#box[
  // Error: 3-32 not allowed here
  #set document(title: "Hello")
]

---
#box[
  // Error: 3-18 not allowed here
  #set page("a4")
]

---
#box[
  // Error: 3-15 not allowed here
  #pagebreak()
]
