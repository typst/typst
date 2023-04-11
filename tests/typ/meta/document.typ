// Test document and page-level styles.

---
// This is okay.
#set document(title: "Hello")
What's up?

---
// This, too.
// Ref: false
#set document(author: ("A", "B"))

---
// This, too.
// Error: 23-29 expected string, found integer
#set document(author: (123,))
What's up?

---
Hello

// Error: 2-30 document set rules must appear before any content
#set document(title: "Hello")

---
// Error: 10-12 can only be used in set rules
#document()

---
#box[
  // Error: 4-32 document set rules are not allowed inside of containers
  #set document(title: "Hello")
]

---
#box[
  // Error: 4-18 page configuration is not allowed inside of containers
  #set page("a4")
]

---
#box[
  // Error: 4-15 pagebreaks are not allowed inside of containers
  #pagebreak()
]
