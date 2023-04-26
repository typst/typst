// Test if text with region works

---
// without any region
#set text(lang: "zh")
#outline()

---
// with unknown region configured
#set text(lang: "zh", region: "XX")
#outline()

---
// with region configured
#set text(lang: "zh", region: "TW")
#outline()
