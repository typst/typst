// Test setting the document language.

--- text-lang paged html pdftags ---
// without any region
#set text(font: "Noto Serif CJK TC", lang: "zh")
#outline()

--- text-lang-unknown-region paged html pdftags ---
// with unknown region configured
#set text(font: "Noto Serif CJK TC", lang: "zh", region: "XX")
#outline()

--- text-lang-region paged html pdftags ---
// with region configured
#set text(font: "Noto Serif CJK TC", lang: "zh", region: "TW")
#outline()

--- text-lang-hyphenate paged ---
// Ensure that setting the language does have effects.
#set text(hyphenate: true)
#grid(
  columns: 2 * (20pt,),
  gutter: 1fr,
  text(lang: "en")["Eingabeaufforderung"],
  text(lang: "de")["Eingabeaufforderung"],
)

--- text-lang-shaping paged ---
// Test that the language passed to the shaper has an effect.
#set text(font: "Ubuntu")

// Some lowercase letters are different in Serbian Cyrillic compared to other
// Cyrillic languages. Since there is only one set of Unicode codepoints for
// Cyrillic, these can only be seen when setting the language to Serbian and
// selecting one of the few fonts that support these letterforms.
Бб
#text(lang: "uk")[Бб]
#text(lang: "sr")[Бб]

--- text-lang-script-shaping paged ---
// Verify that writing script/language combination has an effect
#{
  set text(size:20pt)
  set text(script: "latn", lang: "en")
  [Ş ]
  set text(script: "latn", lang: "ro")
  [Ş ]
  set text(script: "grek", lang: "ro")
  [Ş ]
}

--- text-script-bad-type paged ---
// Error: 19-23 expected string or auto, found none
#set text(script: none)

--- text-script-bad-value paged ---
// Error: 19-23 expected three or four letter script code (ISO 15924 or 'math')
#set text(script: "ab")

--- text-lang-bad-type paged ---
// Error: 17-21 expected string, found none
#set text(lang: none)

--- text-lang-bad-value paged ---
// Error: 17-20 expected two or three letter language code (ISO 639-1/2/3)
#set text(lang: "ӛ")

--- text-lang-bad-value-emoji paged ---
// Error: 17-20 expected two or three letter language code (ISO 639-1/2/3)
#set text(lang: "😃")

--- text-region-bad-value paged ---
// Error: 19-24 expected two letter region code (ISO 3166-1 alpha-2)
#set text(region: "hey")

--- text-language-fallback-english paged ---
#set text(lang: "qaa")
#outline()
#set text(lang: "qaa", region: "aa")
#outline()

--- text-lang-hint-region-parameter paged ---
// Error: 17-24 expected two or three letter language code (ISO 639-1/2/3)
// Hint: 17-24 you should leave only "en" in the `lang` parameter and specify "gb" in the `region` parameter
#set text(lang: "en-gb")
