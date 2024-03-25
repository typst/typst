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
#set text(lang: "ja", font: ("Linux Libertine", "Noto Serif CJK JP"))
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

---
// Test punctuation marks adjustment in justified paragraph.

// The test case includes the following scenarios:
// - Compression of punctuation marks at line start or line end
// - Adjustment of adjacent punctuation marks

#set page(width: 110pt + 10pt, margin: (x: 5pt))
#set text(lang: "zh", font: "Noto Serif CJK SC")
#set par(justify: true)

标注在字间的标点符号（乙式括号省略号以外）通常占一个汉字宽度，使其易于识别、适合配置及排版，有些排版风格完全不对标点宽度进行任何调整。但是为了让文字体裁更加紧凑易读，，，以及执行3.1.4 行首行尾禁则时，就需要对标点符号的宽度进行调整。是否调整取决于……
