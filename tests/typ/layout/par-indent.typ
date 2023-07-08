// Test paragraph indent.

---
#set par(first-line-indent: 12pt, leading: 5pt)
#set block(spacing: 5pt)
#show heading: set text(size: 10pt)

The first paragraph has no indent.

But the second one does.

#box(image("/files/tiger.jpg", height: 6pt))
starts a paragraph, also with indent.

#align(center, image("/files/rhino.png", width: 1cm))

= Headings
- And lists.
- Have no indent.

  Except if you have another paragraph in them.

#set text(8pt, lang: "ar", font: ("Noto Sans Arabic", "Linux Libertine"))
#set par(leading: 8pt)

= Arabic
دع النص يمطر عليك

ثم يصبح النص رطبًا وقابل للطرق ويبدو المستند رائعًا.

---
// This is madness.
#set par(first-line-indent: 12pt)
Why would anybody ever ...

... want spacing and indent?

---
// Test hanging indent.
#set par(hanging-indent: 15pt, justify: true)
#lorem(10)

---
#set par(hanging-indent: 1em)
Welcome \ here. Does this work well?

---
#set par(hanging-indent: 2em)
#set text(dir: rtl)
لآن وقد أظلم الليل وبدأت النجوم
تنضخ وجه الطبيعة التي أعْيَتْ من طول ما انبعثت في النهار
