--- html-document-multiple-authors html ---
// Should emit multiple <meta> tags.
#set document(author: ("John Doe", "Jane Doe"))

--- html-document-multiple-keywords html ---
// Should emit a single <meta> tag.
#set document(keywords: ("foo", "bar"))
