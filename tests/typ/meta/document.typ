// Test document and page-level styles.

---
// This is okay.
#set document(title: "Hello")
What's up?

---
Hello

// Error: 2-30 must appear before any content
#set document(title: "Hello")

---
#box[
  // Error: 4-32 not allowed here
  #set document(title: "Hello")
]

---
#box[
  // Error: 4-18 not allowed here
  #set page("a4")
]

---
#box[
  // Error: 4-15 not allowed here
  #pagebreak()
]
