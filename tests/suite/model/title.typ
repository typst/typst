// Test title element.

--- title-basic render html ---
#title[Some Title]

--- title-and-heading render html ---
#title([A cool title])
= Some level one heading

--- title-show-rule ---
#show title: set text(3em)
#title[Some Title]
