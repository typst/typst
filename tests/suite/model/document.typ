// Test document and page-level styles.

--- document-set-title render ---
#set document(title: [Hello])
What's up?

--- document-set-author-date render ---
#set document(author: ("A", "B"), date: datetime.today())

--- document-date-bad render ---
// Error: 21-28 expected datetime, none, or auto, found string
#set document(date: "today")

--- document-author-bad render ---
// Error: 23-29 expected string, found integer
#set document(author: (123,))
What's up?

--- document-set-after-content render ---
// Document set rules can appear anywhere in top-level realization, also after
// content.
Hello
#set document(title: [Hello])

--- document-constructor render ---
// Error: 2-12 can only be used in set rules
#document()

--- document-set-in-container render ---
#box[
  // Error: 4-32 document set rules are not allowed inside of containers
  #set document(title: [Hello])
]

--- issue-4065-document-context render ---
// Test that we can set document properties based on context.
#show: body => context {
  let all = query(heading)
  let title = if all.len() > 0 { all.first().body }
  set document(title: title)
  body
}

#show heading: none
= Top level

--- issue-4769-document-context-conditional render ---
// Test that document set rule can be conditional on document information
// itself.
#set document(author: "Normal", title: "Alternative")
#context {
  set document(author: "Changed") if "Normal" in document.author
  set document(title: "Changed") if document.title ==  "Normal"
}
