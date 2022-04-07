
---
#set page(width: 180pt)
#set par(
  lang: "en",
  justify: true,
  indent: 14pt,
  spacing: 0pt,
  leading: 5pt,
)

This text is justified, meaning that spaces are stretched so that the text
forms a "block" with flush edges at both sides.

First line indents and hyphenation play nicely with justified text.

---
// Test that lines with hard breaks aren't justified.
#set par(justify: true)
A B C \
D

---
// Test that justificating chinese text is at least a bit sensible.
#set page(width: 200pt)
#set par(justify: true)
中文维基百科使用汉字书写，汉字是汉族或华人的共同文字，是中国大陆、新加坡、马来西亚、台湾、香港、澳门的唯一官方文字或官方文字之一。25.9%，而美国和荷兰则分別占13.7%及8.2%。近年來，中国大陆地区的维基百科编辑者正在迅速增加；
