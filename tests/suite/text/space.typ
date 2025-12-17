// Test whitespace handling.

--- space-collapsing paged ---
// Spacing around code constructs.
A#let x = 1;B  #test(x, 1) \
C #let x = 2;D #test(x, 2) \
E#if true [F]G \
H #if true{"I"} J \
K #if true [L] else []M \
#let c = true; N#while c [#(c = false)O] P \
#let c = true; Q #while c { c = false; "R" } S \
T#for _ in (none,) {"U"}V
#let foo = "A" ; \
#foo;B \
#foo; B \
#foo ;B

--- space-collapsing-comments paged ---
// Test spacing with comments.
A/**/B/**/C \
A /**/ B/**/C \
A /**/B/**/ C

--- space-collapsing-with-h paged ---
// Test spacing collapsing before spacing.
#set align(right)
A #h(0pt) B #h(0pt) \
A B \
A #h(-1fr) B

--- text-font-just-a-space paged ---
// Test that a run consisting only of whitespace isn't trimmed.
A#text(font: "IBM Plex Serif")[ ]B

--- text-font-change-after-space paged ---
// Test font change after space.
Left #text(font: "IBM Plex Serif")[Right].

--- space-collapsing-linebreaks paged ---
// Test that linebreak consumed surrounding spaces.
#align(center)[A \ B \ C]

--- space-collapsing-stringy-linebreak paged ---
// Test that space at start of non-backslash-linebreak line isn't trimmed.
A#"\n" B

--- space-trailing-linebreak paged ---
// Test that trailing space does not force a line break.
LLLLLLLLLLLLLLLLLL R _L_

--- space-ideographic-kept paged ---
// Test that ideographic spaces are preserved.
#set text(lang: "ja", font: "Noto Serif CJK JP")

だろうか？　何のために！　私は、

--- space-thin-kept paged ---
// Test that thin spaces are preserved.
| | U+0020 regular space \
| | U+2009 thin space

--- space-eq-newline paged ---
// Test whether spaces with/without newlines compare equal.
#let parbreak = [

]
#let one-newline = [
]
#let no-newline = [ ]
// parbreak is not equal
#assert.ne(one-newline, parbreak)
// spaces are equal despite newlines
#assert.eq(one-newline, no-newline)

--- issue-792-newline-space-discarding paged ---
// Test whether spaces with/without newlines are discarded adjacent to
// Chinese/Japanese text.

// Discard spaces from just a newline/comment
换
行

注//
释

多行/*
*/注释

// Keep spaces from a space character
空 格

// With both a space and a newline it still discards
空格 //
注释

// Even if the spaces look like this
水
/**/ 果

--- newline-space-discarding-punctuation paged ---
#set page(width: auto)
// We collapse spaces next to any fullwidth punctuation
你好，
你好。
“你好？”
你好。

// But not if the punctuation is ambiguous on both sides
“你好”
“你好”

--- newline-space-discarding-korean paged ---
// Korean doesn't collapse spaces on newlines
줄
바꿈

// Unless using fullwidth punctuation
쉼표，
줄 바꿈

--- newline-space-discarding-strong paged ---
// Test newline space discarding with strong emphasis.
*空 *格

*换*
行

// This space still collapses because it is followed by a newline space.
*空格 *
换行

// The second space here also collapses because it follows a newline space.
空格
* 换行*

--- newline-space-discarding-regex-show-rule paged ---
// Test newline space discarding with regex show rules.
#show regex("注 释|换行 newline|newline 换行"): set text(red)
#show regex("注释|换行newline|newline换行"): set text(blue)

注//
释

// Chinese + Latin does collapse the space
换行
newline

// Latin + Chinese does collapse the space
newline
换行

--- newline-space-discarding-dynamic paged ---
// Test newline space discarding with dynamic variables.
#let foo = [水果] // collapses
#foo
#foo

#let foo = [fruit] // doesn't collapse
#foo
#foo

#let one-newline = [
]
#let no-newline = [ ]
啊#one-newline;啊 // collapses

啊#no-newline;啊 // doesn't collapse

--- newline-space-discarding-edge-cases paged ---
// Test newline space discarding for edge case characters.
// Characters inspired by clreq and jlreq:
// https://www.w3.org/TR/clreq
// https://www.w3.org/TR/jlreq

// Whether each string should discard an adjacent newline space.
#let should-discard = (
  // Basic characters in different languages
  ("A", false),
  ("漢", true),
  ("汉", true),
  ("あ", true),
  ("ア", true),
  ("가", false),
  ("ힰ", false),
  // Emoji don't discard spaces
  ("😀", false),
  ("🏳️‍🌈", false),
  // Miscellaneous
  ("①", false),
  ("⊕", false),
  ("　", true),
  ("ー", true),
  ("₩", true), // Firefox excludes this specifically.
  ("￥", true),
  // Ideographic punctuation
  ("。", true),
  ("．", true),
  ("，", true),
  ("、", true),
  ("：", true),
  ("；", true),
  ("！", true),
  ("？", true),
  ("～", true),
  ("・", true),
  ("／", true),
  ("「", true),
  ("」", true),
  ("『", true),
  ("』", true),
  ("（", true),
  ("）", true),
  ("《", true),
  ("》", true),
  ("〈", true),
  ("〉", true),
  ("【", true),
  ("】", true),
  ("〖", true),
  ("〗", true),
  ("〔", true),
  ("〕", true),
  ("［", true),
  ("］", true),
  ("｛", true),
  ("｝", true),
  ("＿", true),
  ("﹏", true),
  // Not these punctuation though
  ("‼", false),
  ("⁇", false),
  ("⸺", false),
  ("-", false),
  ("–", false),
  ("—", false),
  ("·", false),
  ("‧", false),
  ("/", false),
  ("“", false),
  ("”", false),
  ("‘", false),
  ("’", false),
  ("●", false),
  ("•", false),
  // Nor these multi-character punctuation
  ("——", false),
  ("……", false),
  ("⋯⋯", false),
)

#let newline-space = [
]
#let incorrect = state("incorrect", ())
#for (string, discard) in should-discard {
  // Use text show rules to determine if the newline-space was discarded.
  show string + " " + string: if discard {
    incorrect.update(i => i + (string,))
  }
  show string + "" + string: if not discard {
    incorrect.update(i => i + (string,))
  }
  string + newline-space + string
}

#context test((), incorrect.final())
