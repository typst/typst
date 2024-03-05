#set page(width: 15em)

// In the following example, the space between 》！ and ？ should be squeezed.
// because zh-CN follows GB style
#set text(lang: "zh", region: "CN", font: "Noto Serif CJK SC")
原来，你也玩《原神》！？

// However, in the following example, the space between 》！ and ？ should not be squeezed.
// because zh-TW does not follow GB style
#set text(lang: "zh", region: "TW", font: "Noto Serif CJK TC")
原來，你也玩《原神》！ ？

#set text(lang: "zh", region: "CN", font: "Noto Serif CJK SC")
「真的吗？」

#set text(lang: "ja", font: "Noto Serif CJK JP")
「本当に？」
---

#set text(lang: "zh", region: "CN", font: "Noto Serif CJK SC")
《书名〈章节〉》 // the space between 〉 and 》 should be squeezed

〔茸毛〕：很细的毛 // the space between 〕 and ： should be squeezed

---
#set page(width: 21em)
#set text(lang: "zh", region: "CN", font: "Noto Serif CJK SC")

// These examples contain extensive use of Chinese punctuation marks,
// from 《Which parentheses should be used when applying parentheses?》.
// link: https://archive.md/2bb1N


（〔中〕医、〔中〕药、技）系列评审

（长三角［长江三角洲］）（GB/T 16159—2012《汉语拼音正词法基本规则》）

【爱因斯坦（Albert Einstein）】物理学家

〔（2009）民申字第1622号〕

“江南海北长相忆，浅水深山独掩扉。”（［唐］刘长卿《会赦后酬主簿所问》）

参看1378页〖象形文字〗。（《现代汉语词典》修订本）
