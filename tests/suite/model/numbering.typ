// Test integrated numbering patterns.

--- numbering ---
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

// Symbols.
#t(pat: "*", "-", "*", "†", "‡", "§", "¶", "‖", "**")

// Hebrew.
#t(pat: "א", step: 2, 9, "ט׳", "י״א", "י״ג")

// Chinese.
#t(pat: "一", step: 2, 9, "九", "十一", "十三", "十五", "十七", "十九")
#t(pat: "壹", step: 2, 9, "玖", "拾壹", "拾叁", "拾伍", "拾柒", "拾玖")

// Japanese.
#t(pat: "イ", "-", "イ", "ロ", "ハ", 47, "ス", "イイ", "イロ", "イハ", 2256, "スス", "イイイ")
#t(pat: "い", "-", "い", "ろ", "は", 47, "す", "いい", "いろ", "いは")
#t(pat: "あ", "-", "あ", "い", "う", "え", "お", "か", "き", "く")
#t(pat: "ア", "-", "ア", "イ", "ウ", "エ", "オ", "カ", "キ", "ク")

// Korean.
#t(pat: "가", "-", "가", "나", "다", 47, "다마", "다바", "다사", "다아")
#t(pat: "ㄱ", "-", "ㄱ", "ㄴ", "ㄷ", 47, "ㄷㅁ")

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

// Circled number.
#t(pat: "①", 1, "①")
#t(pat: "①", 50, "㊿")

// Double-circled number.
#t(pat: "⓵", 1, "⓵")
#t(pat: "⓵", 10, "⓾")

--- numbering-negative ---
// Error: 17-19 number must be at least zero
#numbering("1", -1)
