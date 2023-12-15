// https://github.com/typst/typst/issues/2268
// The augment line should be of the same color as the text
#set text(
  font: "New Computer Modern",
  lang: "en",
  fill: yellow,
)

$mat(augment: #1, M, v) arrow.r.squiggly mat(augment: #1, R, b)$
