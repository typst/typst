// Test paragraph indent.

---
#set par(indent: 12pt, leading: 5pt)
#set block(spacing: 5pt)
#show heading: text.with(size: 10pt)

The first paragraph has no indent.

But the second one does.

#image("/res/tiger.jpg", height: 6pt)
starts a paragraph without indent.

#align(center, image("/res/rhino.png", width: 1cm))

= Headings
- And lists.
- Have no indent.

  Except if you have another paragraph in them.

#set text(8pt, lang: "ar", "Noto Sans Arabic", "IBM Plex Sans")
#set par(leading: 8pt)

= Arabic
دع النص يمطر عليك

ثم يصبح النص رطبًا وقابل للطرق ويبدو المستند رائعًا.


---
// This is madness.
#set par(indent: 12pt)
Why would anybody ever ...

... want spacing and indent?
