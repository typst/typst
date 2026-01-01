// CJK Breaking Property Test
// Validation of Typst's cjk-breaking option (keep-all vs distribute) behavior

--- cjk-breaking-basic paged ---

#set page(paper: "a4", margin: 2cm)
#set text(
  size: 11pt,
  lang: "en",
  font: (
    "Times New Roman",
    "SimSun",           // Chinese (Simplified)
    "SimHei",           // Chinese (Simplified)
    "Microsoft YaHei",  // Chinese (Simplified)
    "Malgun Gothic",    // Korean
    "Apple SD Gothic Neo", // Korean
    "Hiragino Sans",    // Japanese
    "Yu Gothic",        // Japanese
  )
)
#set heading(numbering: "1.")

= CJK Breaking Option Test

This document validates that Typst's `cjk-breaking` property works correctly 
with Chinese, Korean, Japanese, and English text.

== Test Case Definitions

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
    text: "これはにほんごのテキストのかいぎょうほうしきをテストするためのサンプルテキストですひらがなとカタカナがこんざいしています",
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

== Visual Comparison Test

#table(
  columns: (12em, 1fr, 1fr),
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

#pagebreak()

== Automated Validation Test

#context {
  let results = ()
  
  for case in test-cases {
    let t-keep-all = text(cjk-breaking: "keep-all", case.text)
    let t-distribute = text(cjk-breaking: "distribute", case.text)
    
    let m-keep-all = measure(t-keep-all, width: case.width)
    let m-distribute = measure(t-distribute, width: case.width)
    
    // keep-all does not break inside words, so if text is too long,
    // it may not fit on a single line
    // distribute breaks at character level, so more lines may be created
    
    let height-diff = m-distribute.height - m-keep-all.height
    
    results.push((
      name: case.name,
      keep-all-height: m-keep-all.height,
      distribute-height: m-distribute.height,
      height-diff: height-diff
    ))
  }
  
  table(
    columns: (1fr, auto, auto, auto),
    stroke: 0.5pt + gray,
    inset: 0.3em,
    table.header(
      [*Test Case*],
      [*keep-all Height*],
      [*distribute Height*],
      [*Difference*]
    ),
    ..for result in results {
      (
        [*#result.name*],
        [#result.keep-all-height],
        [#result.distribute-height],
        [#result.height-diff]
      )
    }
  )
}

== Expected Behavior

- *keep-all*: Keeps words or phrases together on a single line as much as possible. 
  If text exceeds the box width, overflow may occur.
  For CJK characters (Chinese, Korean, Japanese), word boundaries are not clear, 
  so this option may provide more natural line breaking.

- *distribute*: Allows text to break at character level. 
  Uses space efficiently but may not respect word boundaries.
  For English, line breaking typically occurs at word boundaries, 
  but for CJK characters, it may break at character level.

== Test Results Interpretation

In the table above:
- The left column shows the name and description of each test case.
- The middle column shows results using the `keep-all` option.
- The right column shows results using the `distribute` option.
- You can visually compare the line breaking behavior differences between the two options.

Notable points:
- *CJK Characters* (Chinese, Korean, Japanese): `keep-all` tries not to separate words, 
  but `distribute` can break at character level.
- *English*: Line breaking typically occurs at word boundaries, 
  and the difference between the two options may be less pronounced.
