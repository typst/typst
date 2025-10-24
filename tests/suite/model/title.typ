// Test title element.

--- title render html pdftags ---
#set document(title: "My title")
#title()
= A level one heading

--- title-with-body render html ---
#set document(title: "My title")
#title[My display title]

--- title-with-body-auto render ---
#set document(title: "My title")
#title(auto)

--- title-show-set render ---
#show title: set text(blue)
#title[A blue title]

--- title-unset render ---
// Error: 2-9 document title was not set
// Hint: 2-9 set the title with `set document(title: [...])`
// Hint: 2-9 or provide an explicit body with `title[..]`
#title()
