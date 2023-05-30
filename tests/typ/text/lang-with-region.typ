// Test if text with region works

---
// without any region
#set text(font: "Noto Serif CJK TC", lang: "zh")
#outline()

---
// with unknown region configured
#set text(font: "Noto Serif CJK TC", lang: "zh", region: "XX")
#outline()

---
// with region configured
#set text(font: "Noto Serif CJK TC", lang: "zh", region: "TW")
#outline()
