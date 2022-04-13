// Test smart quotes.

---
#set page(width: 200pt)

// Test simple quotations in various languages.
#set text(lang: "en")
"The horse eats no cucumber salad" was the first sentence ever uttered on the 'telephone.'

#set text(lang: "de")
"Das Pferd frisst keinen Gurkensalat" war der erste jemals am 'Fernsprecher' gesagte Satz.

#set text(lang: "fr")
"Le cheval ne mange pas de salade de concombres" est la première phrase jamais prononcée au 'téléphone'.

#set text(lang: "fi")
"Hevonen ei syö kurkkusalaattia" oli ensimmäinen koskaan 'puhelimessa' lausuttu lause.

#set text(lang: "ro")
"Calul nu mănâncă salată de castraveți" a fost prima propoziție rostită vreodată la 'telefon'.

#set text(lang: "ru")
"Лошадь не ест салат из огурцов" - это была первая фраза, сказанная по 'телефону'.

---
// Test single pair of quotes.
#set text(lang: "en")
""

---
// Test sentences with numbers and apostrophes.
#set text(lang: "en")
The 5'11" 'quick' brown fox jumps over the "lazy" dog's ear.

He said "I'm a big fella."

---
// Test escape sequences.
The 5\'11\" 'quick\' brown fox jumps over the \"lazy" dog\'s ear.

---
// Test turning smart quotes off.
#set text(lang: "en")
He's told some books contain questionable "example text".

#set text(smart-quotes: false)
He's told some books contain questionable "example text".

---
// Test changing properties within text.
#set text(lang: "en")
"She suddenly started speaking french: #text(lang: "fr")['Je suis une banane.']" Roman told me.

Some people's thought on this would be #text(smart-quotes: false)["strange."]
