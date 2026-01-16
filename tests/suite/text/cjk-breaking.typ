--- cjk-breaking-basic paged ---
#let test-cases = (
  (
    name: "Long Korean Sentence (No Spaces)",
    text: "한국어텍스트의줄바꿈방식을테스트하기위한샘플텍스트입니다",
    width: 8em,
    description: "Long Korean sentence without spaces"
  ),
  (
    name: "Long Korean Sentence (With Spaces)",
    text: "한국어 텍스트의 줄바꿈 방식을 테스트하기 위한 샘플 텍스트입니다. 이 문장은 띄어쓰기가 포함된 정상적인 형태의 문장입니다.",
    width: 8em,
    description: "Normal Korean sentence with spaces"
  ),
  (
    name: "Long Chinese Sentence (No Spaces)",
    text: "这是一个用于测试中文文本换行方式的示例文本",
    width: 8em,
    description: "Long Chinese sentence without spaces"
  ),
  (
    name: "Long Chinese Sentence (With Spaces)",
    text: "这是 一个 用于 测试 中文 文本 换行 方式 的 示例 文本。这个 句子 包含了 正常的 标点 符号 和 空格。",
    width: 8em,
    description: "Chinese sentence with spaces between words for testing"
  ),
  (
    name: "Long Japanese Sentence (No Spaces)",
    text: "これはにほんごのテキストのかいぎょうをテストするためのサンプルです",
    width: 8em,
    description: "Long Japanese sentence without spaces (Hiragana and Katakana only)"
  ),
  (
    name: "Long Japanese Sentence (With Spaces)",
    text: "これは にほんごの テキストの かいぎょうほうしきを テストするための サンプルテキストです。ひらがなと カタカナが こんざいしています。",
    width: 8em,
    description: "Normal Japanese sentence with spaces"
  ),
  (
    name: "Long English Sentence",
    text: "This is a sample English text to demonstrate text wrapping and line breaking behavior in Typst",
    width: 8em,
    description: "Long English sentence with spaces (word-level line breaking)"
  ),
  (
    name: "Mixed Text",
    text: "한글과English와123숫자가섞인텍스트입니다",
    width: 6em,
    description: "Text with mixed character types"
  ),
)

#table(
  columns: (12em, 15em, 15em),
  stroke: 1pt + black,
  inset: 0.5em,
  table.header(
    table.cell(fill: rgb("e8e8e8"), align: center)[*Test Case*],
    table.cell(fill: rgb("e8e8e8"), align: center)[*keep-all*],
    table.cell(fill: rgb("e8e8e8"), align: center)[*distribute*]
  ),
  ..for case in test-cases {
    (
      table.cell(
        align: left,
        fill: rgb("f9f9f9"),
        stroke: (top: 0.5pt + gray),
      )[
        *#case.name* \
        #text(size: 8pt, fill: gray)[#case.description]
      ],
      table.cell(
        stroke: (top: 0.5pt + gray),
      )[
        #set text(cjk-breaking: "keep-all")
        #case.text
      ],
      table.cell(
        stroke: (top: 0.5pt + gray),
      )[
        #set text(cjk-breaking: "distribute")
        #case.text
      ],
    )
  },
)