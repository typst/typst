// Test line breaks.

--- linebreak-overflow render ---
// Test overlong word that is not directly after a hard break.
This is a spaceexceedinglylongy.

--- linebreak-overflow-double render ---
// Test two overlong words in a row.
Supercalifragilisticexpialidocious Expialigoricmetrioxidation.

--- linebreak-hyphen-nbsp render ---
// Test for non-breaking space and hyphen.
There are non\u{2011}breaking~characters.

--- linebreak-narrow-nbsp render ---
// Test for narrow non-breaking space.
#show "_": sym.space.nobreak.narrow
0.1_g, 1_g, 10_g, 100_g, 1_000_g, 10_000_g, 100_000_g, 1_000_000_g

--- linebreak-shape-run render ---
// Test that there are no unwanted line break opportunities on run change.
This is partly emp#emph[has]ized.

--- linebreak-manual render ---
Hard #linebreak() break.

--- linebreak-manual-directly-after-automatic render ---
// Test hard break directly after normal break.
Hard break directly after \ normal break.

--- linebreak-manual-consecutive render ---
// Test consecutive breaks.
Two consecutive \ \ breaks and three \ \ more.

--- linebreak-manual-trailing-multiple render ---
// Test forcing an empty trailing line.
Trailing break \ \

--- linebreak-manual-justified render ---
// Test justified breaks.
#set par(justify: true)
With a soft #linebreak(justify: true)
break you can force a break without #linebreak(justify: true)
breaking justification. #linebreak(justify: false)
Nice!

--- linebreak-whitespace-trimming render ---
// Ensure that even spaces across multiple layout items are trimmed during
// line breaking.
#block(width: 15pt, box(fill: aqua, underline("A   " + text(fill: blue, " ") + "    B")))

--- issues-7168-linebreak-whitespace-trimming-justify-rtl render ---
#set page(width: 80pt)
#set par(justify: true)
#set text(dir: rtl)

Hello From Earth

--- linebreak-thai render ---
// Test linebreak for East Asian languages
ทีวีตรวจทานนอร์ทแฟรีเลคเชอร์โกลด์อัลบัมเชอร์รี่เย้วสโตร์กฤษณ์เคลมเยอบีร่าพ่อค้าบลูเบอร์รี่สหัสวรรษโฮปแคนูโยโย่จูนสตรอว์เบอร์รีซื่อบื้อเยนแบ็กโฮเป็นไงโดนัททอมสเตริโอแคนูวิทย์แดรี่โดนัทวิทย์แอปพริคอทเซอร์ไพรส์ไฮบริดกิฟท์อินเตอร์โซนเซอร์วิสเทียมทานโคโยตี้ม็อบเที่ยงคืนบุญคุณ

--- linebreak-cite-punctuation render ---
// Test punctuation after citations.
#set page(width: 162pt)

They can look for the details in @netwok,
which is the authoritative source.

#bibliography("/assets/bib/works.bib")

--- linebreak-math-punctuation render ---
// Test punctuation after math equations.
#set page(width: 85pt)

We prove $1 < 2$. \
We prove $1 < 2$! \
We prove $1 < 2$? \
We prove $1 < 2$, \
We prove $1 < 2$; \
We prove $1 < 2$: \
We prove $1 < 2$- \
We prove $1 < 2$– \
We prove $1 < 2$— \

--- linebreak-link render ---
#link("https://example.com/(ab") \
#link("https://example.com/(ab)") \
#link("https://example.com/(paren)") \
#link("https://example.com/paren)") \
#link("https://hi.com/%%%%%%%%abcdef") \

--- linebreak-link-justify render ---
#set page(width: 240pt)
#set par(justify: true)

Here's a link https://url.com/data/extern12840%data_urlenc and then there are more
links #link("www.url.com/data/extern12840%data_urlenc") in my text of links
http://mydataurl/hash/12098541029831025981024980124124214/incremental/progress%linkdata_information_setup_my_link_just_never_stops_going/on?query=false

--- linebreak-link-end render ---
// Ensure that there's no unconditional break at the end of a link.
#set page(width: 180pt, height: auto, margin: auto)
#set text(11pt)

For info see #link("https://myhost.tld").

--- issue-2105-linebreak-tofu render ---
#linebreak()中文

--- issue-3082-chinese-punctuation render ---
#set text(font: "Noto Serif CJK TC", lang: "zh")
#set page(width: 230pt)

課有手冬，朱得過已誰卜服見以大您即乙太邊良，因且行肉因和拉幸，念姐遠米巴急（abc0），松黃貫誰。

--- issue-80-emoji-linebreak render ---
// Test that there are no linebreaks in composite emoji (issue #80).
#set page(width: 50pt, height: auto)
#h(99%) 🏳️‍🌈
🏳️‍🌈

--- issue-hyphenate-in-link render ---
#set par(justify: true)

// The `linebreak()` function accidentally generated out-of-order breakpoints
// for links because it now splits on word boundaries. We avoid the link markup
// syntax because it's show rule interferes.
#"http://creativecommons.org/licenses/by-nc-sa/4.0/"

--- issue-4468-linebreak-thai render ---
// In this bug, empty-range glyphs at line break boundaries could be duplicated.
// This happens for Thai specifically because it has both
// - line break opportunities
// - shaping that results in multiple glyphs in the same cluster
#set text(font: "Noto Sans Thai")
#h(85pt) งบิก

--- issue-5235-linebreak-optimized-without-justify render ---
#set page(width: 207pt, margin: 15pt)
#set text(11pt)

#set par(linebreaks: "simple")
Some texts feature many longer
words. Those are often exceedingly
challenging to break in a visually
pleasing way.

#set par(linebreaks: "optimized")
Some texts feature many longer
words. Those are often exceedingly
challenging to break in a visually
pleasing way.

--- issue-5489-matrix-stray-linebreak render ---
#table(
  columns: (70pt,) * 1,
  align: horizon + center,
  stroke: 0.6pt,
  [$mat(2241/2210,-71/1105;-71/1105,147/1105)$],
)

--- linebreak-default-ignorables render ---
#set text(font: "Noto Sans Math")
\u{2295}\u{FE00} vs \u{2295}\u{FE00}
