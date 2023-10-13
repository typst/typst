// Test Chinese text in narrow lines.

// In Chinese typography, line length should be multiples of the character size
// and the line ends should be aligned with each other.
// Most Chinese publications do not use hanging punctuation at line end.
#set page(width: auto)
#set par(justify: true)
#set text(lang: "zh", font: "Noto Serif CJK SC")

#rect(inset: 0pt, width: 80pt, fill: rgb("eee"))[
  中文维基百科使用汉字书写，汉字是汉族或华人的共同文字，是中国大陆、新加坡、马来西亚、台湾、香港、澳门的唯一官方文字或官方文字之一。25.9%，而美国和荷兰则分別占13.7%及8.2%。近年來，中国大陆地区的维基百科编辑者正在迅速增加；
]

---
// Japanese typography is more complex, make sure it is at least a bit sensible.
#set page(width: auto)
#set par(justify: true)
#set text(lang: "jp", font: ("Linux Libertine", "Noto Serif CJK JP"))
#rect(inset: 0pt, width: 80pt, fill: rgb("eee"))[
  ウィキペディア（英: Wikipedia）は、世界中のボランティアの共同作業によって執筆及び作成されるフリーの多言語インターネット百科事典である。主に寄付に依って活動している非営利団体「ウィキメディア財団」が所有・運営している。

  専門家によるオンライン百科事典プロジェクトNupedia（ヌーペディア）を前身として、2001年1月、ラリー・サンガーとジミー・ウェールズ（英: Jimmy Donal "Jimbo" Wales）により英語でプロジェクトが開始された。
]

---
// Test punctuation whitespace adjustment
#set page(width: auto)
#set text(lang: "zh", font: "Noto Serif CJK SC")
#set par(justify: true)
#rect(inset: 0pt, width: 80pt, fill: rgb("eee"))[
  “引号测试”，还，

  《书名》《测试》下一行

  《书名》《测试》。
]

「『引号』」。“‘引号’”。

---
// Test Variants of Mainland China, Hong Kong, and Japan.

// 17 characters a line.
#set page(width: 170pt + 10pt, margin: (x: 5pt))
#set text(lang: "zh", font: "Noto Serif CJK SC")
#set par(justify: true)

孔雀最早见于《山海经》中的《海内经》：“有孔雀。”东汉杨孚著《异物志》记载，岭南：“孔雀，其大如大雁而足高，毛皆有斑纹彩，捕而蓄之，拍手即舞。”

#set text(lang: "zh", region: "hk", font: "Noto Serif CJK TC")
孔雀最早见于《山海经》中的《海内经》：「有孔雀。」东汉杨孚著《异物志》记载，岭南：「孔雀，其大如大雁而足高，毛皆有斑纹彩，捕而蓄之，拍手即舞。」
