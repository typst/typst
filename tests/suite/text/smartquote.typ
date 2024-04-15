--- smartquote ---
// LARGE
#set page(width: 250pt)

// Test simple quotations in various languages.
#set text(lang: "en")
"The horse eats no cucumber salad" was the first sentence ever uttered on the 'telephone.'

#set text(lang: "de")
"Das Pferd frisst keinen Gurkensalat" war der erste jemals am 'Fernsprecher' gesagte Satz.

#set text(lang: "de", region: "CH")
"Das Pferd frisst keinen Gurkensalat" war der erste jemals am 'Fernsprecher' gesagte Satz.

#set text(lang: "es", region: none)
"El caballo no come ensalada de pepino" fue la primera frase pronunciada por 'teléfono'.

#set text(lang: "es", region: "MX")
"El caballo no come ensalada de pepino" fue la primera frase pronunciada por 'teléfono'.

#set text(lang: "fr", region: none)
"Le cheval ne mange pas de salade de concombres" est la première phrase jamais prononcée au 'téléphone'.

#set text(lang: "fi")
"Hevonen ei syö kurkkusalaattia" oli ensimmäinen koskaan 'puhelimessa' lausuttu lause.

#set text(lang: "gr")
"Το άλογο δεν τρώει αγγουροσαλάτα" ήταν η πρώτη πρόταση που ειπώθηκε στο 'τηλέφωνο'.

#set text(lang: "he")
"הסוס לא אוכל סלט מלפפונים" היה המשפט ההראשון שנאמר ב 'טלפון'.

#set text(lang: "ro")
"Calul nu mănâncă salată de castraveți" a fost prima propoziție rostită vreodată la 'telefon'.

#set text(lang: "ru")
"Лошадь не ест салат из огурцов" - это была первая фраза, сказанная по 'телефону'.

--- smartquote-empty ---
// Test single pair of quotes.
""

--- smartquote-apostrophe ---
// Test sentences with numbers and apostrophes.
The 5'11" 'quick' brown fox jumps over the "lazy" dog's ear.

He said "I'm a big fella."

--- smartquote-escape ---
// Test escape sequences.
The 5\'11\" 'quick\' brown fox jumps over the \"lazy" dog\'s ear.

--- smartquote-disable ---
// Test turning smart quotes off.
He's told some books contain questionable "example text".

#set smartquote(enabled: false)
He's told some books contain questionable "example text".

--- smartquote-disabled-temporarily ---
// Test changing properties within text.
"She suddenly started speaking french: #text(lang: "fr")['Je suis une banane.']" Roman told me.

Some people's thought on this would be #[#set smartquote(enabled: false); "strange."]

--- smartquote-nesting ---
// Test nested double and single quotes.
"'test statement'" \
"'test' statement" \
"statement 'test'"

--- smartquote-custom ---
// Use language quotes for missing keys, allow partial reset
#set smartquote(quotes: "«»")
"Double and 'Single' Quotes"

#set smartquote(quotes: (double: auto, single: "«»"))
"Double and 'Single' Quotes"

--- smartquote-custom-complex ---
// Allow 2 graphemes
#set smartquote(quotes: "a\u{0301}a\u{0301}")
"Double and 'Single' Quotes"

#set smartquote(quotes: (single: "a\u{0301}a\u{0301}"))
"Double and 'Single' Quotes"

--- smartquote-custom-bad-string ---
// Error: 25-28 expected 2 characters, found 1 character
#set smartquote(quotes: "'")

--- smartquote-custom-bad-array ---
// Error: 25-35 expected 2 quotes, found 4 quotes
#set smartquote(quotes: ("'",) * 4)

--- smartquote-custom-bad-dict ---
// Error: 25-45 expected 2 quotes, found 4 quotes
#set smartquote(quotes: (single: ("'",) * 4))

--- issue-3662-pdf-smartquotes ---
// Smart quotes were not appearing in the PDF outline, because they didn't
// implement `PlainText`.
= It's "Unnormal Heading"
= It’s “Normal Heading”

#set smartquote(enabled: false)
= It's "Unnormal Heading"
= It's 'single quotes'
= It’s “Normal Heading”

--- issue-1041-smartquotes-in-outline ---
#set page(width: 15em)
#outline()

= "This" "is" "a" "test"

--- issue-1540-smartquotes-across-newlines ---
// Test that smart quotes are inferred correctly across newlines.
"test"#linebreak()"test"

"test"\
"test"
