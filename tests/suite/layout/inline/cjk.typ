// Test CJK-specific features.

--- text-chinese-basic ---
// Test basic Chinese text from Wikipedia.
#set text(font: "Noto Serif CJK SC")

是美国广播公司电视剧《迷失》第3季的第22和23集，也是全剧的第71集和72集
由执行制作人戴蒙·林道夫和卡尔顿·库斯编剧，导演则是另一名执行制作人杰克·本德
节目于2007年5月23日在美国和加拿大首播，共计吸引了1400万美国观众收看
本集加上插播广告一共也持续有两个小时

--- text-cjk-latin-spacing ---
#set page(width: 50pt + 10pt, margin: (x: 5pt))
#set text(lang: "zh", font: "Noto Serif CJK SC", cjk-latin-spacing: auto)
#set par(justify: true)

中文，中12文1中，文12中文

中文，中ab文a中，文ab中文

#set text(cjk-latin-spacing: none)

中文，中12文1中，文12中文

中文，中ab文a中，文ab中文

--- cjk-punctuation-adjustment-1 ---
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

--- cjk-punctuation-adjustment-2 ---
#set text(lang: "zh", region: "CN", font: "Noto Serif CJK SC")
《书名〈章节〉》 // the space between 〉 and 》 should be squeezed

〔茸毛〕：很细的毛 // the space between 〕 and ： should be squeezed

--- cjk-punctuation-adjustment-3 ---
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

--- issue-2538-cjk-latin-spacing-before-linebreak ---
// Issue #2538
#set text(cjk-latin-spacing: auto)

abc字

abc字#linebreak()

abc字#linebreak()
母

abc字\
母

--- issue-6539-cjk-latin-spacing-at-manual-linebreak ---
// Issue #6539
#set text(cjk-latin-spacing: auto)
#set box(width: 2.3em, stroke: (x: green))

#box(align(end)[甲国\ T国])

#box(align(end)[乙国 \ T国])

#box(align(end)[丙国 T国])

#box(align(end)[丁国T国])

--- issue-2650-cjk-latin-spacing-meta ---
测a试

测#context [a]试

--- issue-7113-cjk-latin-spacing-shift ---
孔乙己#super[1]与上大人#super[2]。

孔乙己#super[A应保留spacing]与上大人#super[B]。

时间#footnote[有空白]

时间#sub[123]#super[时间]B
