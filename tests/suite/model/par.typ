// Test configuring paragraph properties.

--- par-basic ---
#set page(width: 250pt, height: 120pt)

But, soft! what light through yonder window breaks? It is the east, and Juliet
is the sun. Arise, fair sun, and kill the envious moon, Who is already sick and
pale with grief, That thou her maid art far more fair than she: Be not her maid,
since she is envious; Her vestal livery is but sick and green And none but fools
do wear it; cast it off. It is my lady, O, it is my love! O, that she knew she
were! She speaks yet she says nothing: what of that? Her eye discourses; I will
answer it.

I am too bold, 'tis not to me she speaks: Two of the fairest stars in all the
heaven, Having some business, do entreat her eyes To twinkle in their spheres
till they return. What if her eyes were there, they in her head? The brightness
of her cheek would shame those stars, As daylight doth a lamp; her eyes in
heaven Would through the airy region stream so bright That birds would sing and
think it were not night. See, how she leans her cheek upon her hand! O, that I
were a glove upon that hand, That I might touch that cheek!

--- par-leading-and-block-spacing ---
// Test changing leading and spacing.
#set block(spacing: 1em)
#set par(leading: 2pt)
But, soft! what light through yonder window breaks?

It is the east, and Juliet is the sun.

--- par-first-line-indent ---
#set par(first-line-indent: 12pt, leading: 5pt)
#set block(spacing: 5pt)
#show heading: set text(size: 10pt)

The first paragraph has no indent.

But the second one does.

#box(image("/assets/images/tiger.jpg", height: 6pt))
starts a paragraph, also with indent.

#align(center, image("/assets/images/rhino.png", width: 1cm))

= Headings
- And lists.
- Have no indent.

  Except if you have another paragraph in them.

#set text(8pt, lang: "ar", font: ("Noto Sans Arabic", "Linux Libertine"))
#set par(leading: 8pt)

= Arabic
دع النص يمطر عليك

ثم يصبح النص رطبًا وقابل للطرق ويبدو المستند رائعًا.

--- par-spacing-and-first-line-indent ---
// This is madness.
#set par(first-line-indent: 12pt)
Why would anybody ever ...

... want spacing and indent?

--- par-hanging-indent ---
// Test hanging indent.
#set par(hanging-indent: 15pt, justify: true)
#lorem(10)

--- par-hanging-indent-manual-linebreak ---
#set par(hanging-indent: 1em)
Welcome \ here. Does this work well?

--- par-hanging-indent-rtl ---
#set par(hanging-indent: 2em)
#set text(dir: rtl)
لآن وقد أظلم الليل وبدأت النجوم
تنضخ وجه الطبيعة التي أعْيَتْ من طول ما انبعثت في النهار
