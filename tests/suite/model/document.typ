// Test document and page-level styles.

--- document-set-title ---
// This is okay.
#set document(title: [Hello])
What's up?

--- document-set-author-date ---
// This, too.
#set document(author: ("A", "B"), date: datetime.today())

--- document-date-bad ---
// Error: 21-28 expected datetime, none, or auto, found string
#set document(date: "today")

--- document-author-bad ---
// This, too.
// Error: 23-29 expected string, found integer
#set document(author: (123,))
What's up?

--- document-set-after-content ---
Hello

// Error: 2-30 document set rules must appear before any content
#set document(title: [Hello])

--- document-constructor ---
// Error: 2-12 can only be used in set rules
#document()

--- document-set-in-container ---
#box[
  // Error: 4-32 document set rules are not allowed inside of containers
  #set document(title: [Hello])
]
