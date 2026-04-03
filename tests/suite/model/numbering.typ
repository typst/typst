// Test integrated numbering patterns.

--- numbering eval ---
#let t(pat: "1", step: 1, ..vals) = {
  let num = 0
  for val in vals.pos() {
    if type(val) == int {
      num = val
    } else {
      test(numbering(pat, num), val)
      num += step
    }
  }
}

// Arabic.
#t(pat: "1", "0", "1", "2", "3", "4", "5", "6", 107, "107", "108")

// Greek.
#t(
  pat: "α",
  "𐆊", "α", "β", "γ", "δ", "ε", "στ", "ζ", "η", "θ", "ι",
  "ια", "ιβ", "ιγ", "ιδ", "ιε", "ιστ", "ιζ", "ιη", "ιθ", "κ",
  241, "σμα",
  999, "ϡϟθ",
  1005, "͵αε",
  1999, "͵αϡϟθ",
  2999, "͵βϡϟθ",
  3000, "͵γ",
  3398, "͵γτϟη",
  4444, "͵δυμδ",
  5683, "͵εχπγ",
  9184, "͵θρπδ",
  9999, "͵θϡϟθ",
)
#t(
  pat: sym.Alpha,
  "𐆊", "Α", "Β", "Γ", "Δ", "Ε", "ΣΤ", "Ζ", "Η", "Θ", "Ι",
  "ΙΑ", "ΙΒ", "ΙΓ", "ΙΔ", "ΙΕ", "ΙΣΤ", "ΙΖ", "ΙΗ", "ΙΘ", "Κ",
  241, "ΣΜΑ",
)

// Symbols.
#t(pat: "*", 1, "*", "†", "‡", "§", "¶", "‖", "**")

// Hebrew.
#t(pat: "א", step: 2, 9, "ט", "יא", "יג", 15, "טו", 16, "טז")

// Chinese.
#t(pat: "一", step: 2, 9, "九", "十一", "十三", "十五", "十七", "十九")
#t(pat: "壹", step: 2, 9, "玖", "拾壹", "拾叁", "拾伍", "拾柒", "拾玖")

// Japanese.
#t(pat: "イ", 1, "イ", "ロ", "ハ", 47, "ス", "イイ", "イロ", "イハ", 2256, "スス", "イイイ")
#t(pat: "い", 1, "い", "ろ", "は", 47, "す", "いい", "いろ", "いは")
#t(pat: "あ", 1, "あ", "い", "う", "え", "お", "か", "き", "く")
#t(pat: "ア", 1, "ア", "イ", "ウ", "エ", "オ", "カ", "キ", "ク")

// Korean.
#t(pat: "가", 1, "가", "나", "다", 47, "다마", "다바", "다사", "다아")
#t(pat: "ㄱ", 1, "ㄱ", "ㄴ", "ㄷ", 47, "ㄷㅁ")

// Arabic Indic.
#t(pat: "\u{0661}", 1475, "١٤٧٥")
#t(pat: "\u{06F1}", 1475, "۱۴۷۵")

// Devanagari.
#t(pat: "\u{0967}", 1, "१")
#t(pat: "\u{0967}", 10, "१०")
#t(pat: "\u{0967}", 123456789, "१२३४५६७८९")

// Bengali.
#t(pat: "\u{09E7}", 1, "১")
#t(pat: "\u{09E7}", 10, "১০")
#t(pat: "\u{09E7}", 123456789, "১২৩৪৫৬৭৮৯")

// Bengali Consonants.
#t(pat: "\u{0995}", 1, "ক")
#t(pat: "\u{0995}", 32, "হ")
#t(pat: "\u{0995}", 32*2 , "কহ")

// Armenian.
#t(pat: "ա", 1, "ա", "բ", "գ", 10, "ժ", 15, "ժե", 24, "իդ", 2025, "սիե")
#t(pat: "Ա", 1, "Ա", "Բ", "Գ", 10, "Ժ", 15, "ԺԵ", 24, "ԻԴ", 2025, "ՍԻԵ")

// Circled number.
#t(pat: "①", 1, "①")
#t(pat: "①", 50, "㊿")

// Double-circled number.
#t(pat: "⓵", 1, "⓵")
#t(pat: "⓵", 10, "⓾")

--- numbering-negative eval ---
// Error: 17-19 number must be at least zero
#numbering("1", -1)

--- numbering-illegal-zero eval ---
#test(numbering("१", 0), "०")
// Error: 2-19 the numeral system `korean.syllable` cannot represent zero
#numbering("가", 0)

--- numbering-too-high eval ---
// Error: 2-20 the number 51 is too large to be represented with the `arabic.o` numeral system
#numbering("①", 51)

--- enum-numbering-too-high paged ---
#set enum(numbering: "⓵")
// Error: 1-9 the number 11 is too large to be represented with the `arabic.oo` numeral system
11. Test

--- page-numbering-too-high paged ---
// Error: the number 100 is too large to be represented with the `arabic.o` numeral system
#set page(numbering: "①")
#counter(page).update(100)
Hello

--- page-numbering-too-high-pdf pdf ---
// Error: the number 100 is too large to be represented with the `arabic.o` numeral system
// Hint: this happened when trying to write a page number in the PDF metadata
// Explanation:
// The page number is not displayed on the page. Instead, it is only computed to
// be embedded in the PDF metadata so the error is triggered in `typst-pdf`
// instead of `typst-layout`.
#set page(numbering: "①", footer: none)
#counter(page).update(100)
Hello

--- footnote-numbering-too-high paged ---
#set footnote(numbering: "①")
#counter(footnote).update(100)
// Error: 2-12 the number 101 is too large to be represented with the `arabic.o` numeral system
#footnote[]
