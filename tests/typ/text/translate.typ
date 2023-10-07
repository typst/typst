// Test the `translate` element.

---
// Test `translate` with dictionary.
#let language = translate((
  fr: "Français",
  ja: "日本語",
  sv: "Svenska",
))

#set text(lang: "fr")
#language

#set text(lang: "ja")
#language

#set text(lang: "sv")
#language

---
// Test missing language.
#let language = translate((
  fr: "Français",
  ja: "日本語",
  sv: "Svenska",
))

#set text(lang: "es")
// Error: No translation available for es
#language

---
// Test `translate` with function.
#let locale = translate((lang, region) => {
 if region != none [#(lang)-#region] else { lang }
})

#locale

#set text(lang: "de", region: none)
#locale

#set text(lang: "fr", region: "ca")
#locale
