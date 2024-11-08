--- justify ---
#set page(width: 180pt)
#set par(justify: true, first-line-indent: 14pt, spacing: 5pt, leading: 5pt)

This text is justified, meaning that spaces are stretched so that the text
forms a "block" with flush edges at both sides.

First line indents and hyphenation play nicely with justified text.

--- justify-knuth-story ---
// LARGE
#set page(width: auto, height: auto)
#set par(leading: 4pt, justify: true)
#set text(font: "New Computer Modern")

#let story = [
  In olden times when wishing still helped one, there lived a king whose
  daughters were all beautiful; and the youngest was so beautiful that the sun
  itself, which has seen so much, was astonished whenever it shone in her face.
  Close by the king’s castle lay a great dark forest, and under an old lime-tree
  in the forest was a well, and when the day was very warm, the king’s child
  went out into the forest and sat down by the side of the cool fountain; and
  when she was bored she took a golden ball, and threw it up on high and caught
  it; and this ball was her favorite plaything.
]

#let column(title, linebreaks, hyphenate) = {
  rect(inset: 0pt, width: 132pt, fill: rgb("eee"))[
    #set par(linebreaks: linebreaks)
    #set text(hyphenate: hyphenate)
    #strong(title) \ #story
  ]
}

#grid(
  columns: 3,
  gutter: 10pt,
  column([Simple without hyphens], "simple", false),
  column([Simple with hyphens], "simple", true),
  column([Optimized with hyphens], "optimized", true),
)

--- justify-manual-linebreak ---
// Test that lines with hard breaks aren't justified.
#set par(justify: true)
A B C \
D

--- justify-justified-linebreak ---
// Test forced justification with justified break.
A B C #linebreak(justify: true)
D E F #linebreak(justify: true)

--- justify-basically-empty ---
// Test that there are no hick-ups with justification enabled and
// basically empty paragraph.
#set par(justify: true)
#""

--- justify-shrink-last-line ---
// Test that the last line can be shrunk
#set page(width: 155pt)
#set par(justify: true)
This text can be fitted in one line.

--- justify-avoid-runts ---
// Test that runts are avoided when it's not too costly to do so.
#set page(width: 124pt)
#set par(justify: true)
#for i in range(0, 20) {
	"a b c "
}
#"d"

--- justify-no-leading-spaces ---
// Test that justification cannot lead to a leading space
#set par(justify: true)
#set text(size: 12pt)
#set page(width: 45mm, height: auto)

lorem ipsum 1234, lorem ipsum dolor sit amet

#"  leading whitespace should still be displayed"

--- justify-code-blocks ---
// Test that justification doesn't break code blocks
#set par(justify: true)

```cpp
int main() {
  printf("Hello world\n");
  return 0;
}
```

--- justify-chinese ---
// In Chinese typography, line length should be multiples of the character size
// and the line ends should be aligned with each other. Most Chinese
// publications do not use hanging punctuation at line end.
#set page(width: auto)
#set par(justify: true)
#set text(lang: "zh", font: "Noto Serif CJK SC")

#rect(inset: 0pt, width: 80pt, fill: rgb("eee"))[
  中文维基百科使用汉字书写，汉字是汉族或华人的共同文字，是中国大陆、新加坡、马来西亚、台湾、香港、澳门的唯一官方文字或官方文字之一。25.9%，而美国和荷兰则分別占13.7%及8.2%。近年來，中国大陆地区的维基百科编辑者正在迅速增加；
]

--- justify-japanese ---
// Japanese typography is more complex, make sure it is at least a bit sensible.
#set page(width: auto)
#set par(justify: true)
#set text(lang: "ja", font: ("Libertinus Serif", "Noto Serif CJK JP"))
#rect(inset: 0pt, width: 80pt, fill: rgb("eee"))[
  ウィキペディア（英: Wikipedia）は、世界中のボランティアの共同作業によって執筆及び作成されるフリーの多言語インターネット百科事典である。主に寄付に依って活動している非営利団体「ウィキメディア財団」が所有・運営している。

  専門家によるオンライン百科事典プロジェクトNupedia（ヌーペディア）を前身として、2001年1月、ラリー・サンガーとジミー・ウェールズ（英: Jimmy Donal "Jimbo" Wales）により英語でプロジェクトが開始された。
]

--- justify-whitespace-adjustment ---
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

--- justify-variants ---
// Test Variants of Mainland China, Hong Kong, and Japan.

// 17 characters a line.
#set page(width: 170pt + 10pt, margin: (x: 5pt))
#set text(lang: "zh", font: "Noto Serif CJK SC")
#set par(justify: true)

孔雀最早见于《山海经》中的《海内经》：“有孔雀。”东汉杨孚著《异物志》记载，岭南：“孔雀，其大如大雁而足高，毛皆有斑纹彩，捕而蓄之，拍手即舞。”

#set text(lang: "zh", region: "hk", font: "Noto Serif CJK TC")
孔雀最早见于《山海经》中的《海内经》：「有孔雀。」东汉杨孚著《异物志》记载，岭南：「孔雀，其大如大雁而足高，毛皆有斑纹彩，捕而蓄之，拍手即舞。」

--- justify-punctuation-adjustment ---
// Test punctuation marks adjustment in justified paragraph.

// The test case includes the following scenarios:
// - Compression of punctuation marks at line start or line end
// - Adjustment of adjacent punctuation marks

#set page(width: 110pt + 10pt, margin: (x: 5pt))
#set text(lang: "zh", font: "Noto Serif CJK SC")
#set par(justify: true)

标注在字间的标点符号（乙式括号省略号以外）通常占一个汉字宽度，使其易于识别、适合配置及排版，有些排版风格完全不对标点宽度进行任何调整。但是为了让文字体裁更加紧凑易读，，，以及执行3.1.4 行首行尾禁则时，就需要对标点符号的宽度进行调整。是否调整取决于……

--- justify-without-justifiables ---
// Test breaking a line without justifiables.
#set par(justify: true)
#block(width: 1cm, fill: aqua, lorem(2))

--- issue-2419-justify-hanging-indent ---
// Test that combination of justification and hanging indent doesn't result in
// an underfull first line.
#set par(hanging-indent: 2.5cm, justify: true)
#lorem(5)

--- issue-4651-justify-bad-bound ---
// Test that overflow does not lead to bad bounds in paragraph optimization.
#set par(justify: true)
#block(width: 0pt)[A B]

--- issue-5360-unnecessary-hyphenation ---
// Test whether `Formal` would be in one line.
#set par(justify: true)
#table(columns: 1, [Formal])
