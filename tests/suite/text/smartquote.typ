--- smartquote render ---
#set text(lang: "en")
"The horse eats no cucumber salad" was the first sentence ever uttered on the 'telephone.'

--- smartquote-de render ---
#set text(lang: "de")
"Das Pferd frisst keinen Gurkensalat" war der erste jemals am 'Fernsprecher' gesagte Satz.

--- smartquote-de-ch render ---
#set text(lang: "de", region: "CH")
"Das Pferd frisst keinen Gurkensalat" war der erste jemals am 'Fernsprecher' gesagte Satz.

--- smartquote-es render ---
#set text(lang: "es", region: none)
"El caballo no come ensalada de pepino" fue la primera frase pronunciada por 'teléfono'.

--- smartquote-es-mx render ---
#set text(lang: "es", region: "MX")
"El caballo no come ensalada de pepino" fue la primera frase pronunciada por 'teléfono'.

--- smartquote-fr render ---
#set text(lang: "fr")
"Le cheval ne mange pas de salade de concombres" est la première phrase jamais prononcée au 'téléphone'.

--- smartquote-fr-ch render ---
#set text(lang: "fr", region: "CH")
"Le cheval ne mange pas de salade de concombres" est la première phrase jamais prononcée au 'téléphone'.

--- smartquote-fi render ---
#set text(lang: "fi")
"Hevonen ei syö kurkkusalaattia" oli ensimmäinen koskaan 'puhelimessa' lausuttu lause.

--- smartquote-el render ---
#set text(lang: "el")
"Το άλογο δεν τρώει αγγουροσαλάτα" ήταν η πρώτη πρόταση που ειπώθηκε στο 'τηλέφωνο'.

--- smartquote-he render ---
#set text(lang: "he")
"הסוס לא אוכל סלט מלפפונים" היה המשפט ההראשון שנאמר ב'טלפון'.

--- smartquote-ro render ---
#set text(lang: "ro")
"Calul nu mănâncă salată de castraveți" a fost prima propoziție rostită vreodată la 'telefon'.

--- smartquote-ru render ---
#set text(lang: "ru")
"Лошадь не ест салат из огурцов" - это была первая фраза, сказанная по 'телефону'.

--- smartquote-uk render ---
#set text(lang: "uk")
"Кінь не їсть огірковий салат" — перше речення, коли-небудь вимовлене по 'телефону'.

--- smartquote-it render ---
#set text(lang: "it")
"Il cavallo non mangia insalata di cetrioli" è stata la prima frase pronunciata al 'telefono'.

--- smartquote-la render ---
#set text(lang: "la")
#set smartquote(alternative: true)
"Equus cucumeris sem non edit" prima sententia in 'telephono' prolata fuit.

--- smartquote-empty render ---
// Test single pair of quotes.
""

--- smartquote-apostrophe render ---
// Test sentences with numbers and apostrophes.
The 5'11" 'quick' brown fox jumps over the "lazy" dog's ear.

He said "I'm a big fella."

--- smartquote-escape render ---
// Test escape sequences.
The 5\'11\" 'quick\' brown fox jumps over the \"lazy' dog\'s ear.

--- smartquote-slash render ---
// Test that smartquotes can open before non-whitespace if not nested.
"Hello"/"World" \
'"Hello"/"World"' \
""Hello"/"World""

--- smartquote-close-before-letter render ---
// Test that smartquotes can close before alphabetic letters.
Straight "A"s and "B"s

--- smartquote-prime render ---
// Test that primes result after numbers when possible.
A 2" nail. \
'A 2" nail.' \
"A 2" nail."

--- smartquote-bracket render ---
// Test that brackets indicate an opening quote.
"a ["b"] c" \
"a b"c"d e"

--- smartquote-disable render ---
// Test turning smart quotes off.
He's told some books contain questionable "example text".

#set smartquote(enabled: false)
He's told some books contain questionable "example text".

--- smartquote-disabled-temporarily render ---
// Test changing properties within text.
"She suddenly started speaking french: #text(lang: "fr", region: "CH")['Je suis une banane.']" Roman told me.

Some people's thought on this would be #[#set smartquote(enabled: false); "strange."]

--- smartquote-nesting render ---
// Test nested double and single quotes.
"'test statement'" \
"'test' statement" \
"statement 'test'"

--- smartquote-nesting-twice render html ---
When you said _that "he_ surely meant that 'she intended to say "I'm sorry"'", I was quite confused.

'#box[box]'

--- smartquote-inline-block html ---
Applies across #html.span["inline-level] elements".

Does not apply across #html.div["block-level] elements".

--- smartquote-with-embedding-chars render ---
#set text(lang: "fr")
"#"\u{202A}"bonjour#"\u{202C}"" \
#"\u{202A}""bonjour"#"\u{202C}"

--- smartquote-custom render ---
// Use language quotes for missing keys, allow partial reset
#set smartquote(quotes: "«»")
"Double and 'Single' Quotes"

#set smartquote(quotes: (double: auto, single: "«»"))
"Double and 'Single' Quotes"

--- smartquote-custom-complex render ---
// Allow 2 graphemes
#set smartquote(quotes: "a\u{0301}a\u{0301}")
"Double and 'Single' Quotes"

#set smartquote(quotes: (single: "a\u{0301}a\u{0301}"))
"Double and 'Single' Quotes"

--- smartquote-custom-bad-string render ---
// Error: 25-28 expected 2 characters, found 1 character
#set smartquote(quotes: "'")

--- smartquote-custom-bad-array render ---
// Error: 25-35 expected 2 quotes, found 4 quotes
#set smartquote(quotes: ("'",) * 4)

--- smartquote-custom-bad-dict render ---
// Error: 25-45 expected 2 quotes, found 4 quotes
#set smartquote(quotes: (single: ("'",) * 4))

--- issue-3662-pdf-smartquotes render ---
// Smart quotes were not appearing in the PDF outline, because they didn't
// implement `PlainText`.
= It's "Unnormal Heading"
= It’s “Normal Heading”

#set smartquote(enabled: false)
= It's "Unnormal Heading"
= It's 'single quotes'
= It’s “Normal Heading”

--- issue-1041-smartquotes-in-outline render ---
#set page(width: 15em)
#outline()

= "This" "is" "a" "test"

--- issue-1540-smartquotes-across-newlines render ---
// Test that smart quotes are inferred correctly across newlines.
"test"#linebreak()"test"

"test"\
"test"

--- issue-5146-smartquotes-after-equations render ---
$i$'s $i$ 's
