// Test references.

---
#set heading(numbering: "1.")

= Introduction <intro>
See @setup.

== Setup <setup>
As seen in @intro, we proceed.

---
// Error: 1-5 label does not exist in the document
@foo

---
= First <foo>
= Second <foo>

// Error: 1-5 label occurs multiple times in the document
@foo
