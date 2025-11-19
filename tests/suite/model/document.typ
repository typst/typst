// Test document and page-level styles.

--- document-set-title paged ---
#set document(title: [Hello])
What's up?

--- document-set-author-date paged ---
#set document(author: ("A", "B"), date: datetime.today())

--- document-date-bad paged ---
// Error: 21-28 expected datetime, none, or auto, found string
#set document(date: "today")

--- document-author-bad paged ---
// Error: 23-29 expected string, found integer
#set document(author: (123,))
What's up?

--- document-set-after-content paged ---
// Document set rules can appear anywhere in top-level realization, also after
// content.
Hello
#set document(title: [Hello])

--- document-constructor paged ---
// Error: 2-12 can only be used in set rules
#document()

--- document-set-in-container paged ---
#box[
  // Error: 4-32 document set rules are not allowed inside of containers
  #set document(title: [Hello])
]

--- issue-4065-document-context paged ---
// Test that we can set document properties based on context.
#show: body => context {
  let all = query(heading)
  let title = if all.len() > 0 { all.first().body }
  set document(title: title)
  body
}

#show heading: none
= Top level

--- issue-4769-document-context-conditional paged ---
// Test that document set rule can be conditional on document information
// itself.
#set document(author: "Normal", title: "Alternative")
#context {
  set document(author: "Changed") if "Normal" in document.author
  set document(title: "Changed") if document.title ==  "Normal"
}
