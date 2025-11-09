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

--- issue-792-space-collapsing-cj paged ---
// Test how spaces with/without newlines collapse in Chinese/Japanese text.

// No space from just a newline/comment
换
行

注//
释

多行/*
*/注释

// Should have a space from a space character
空 格

// With both a space and a newline it still collapses
空格 //
注释

--- space-collapsing-cjk-punctuation paged ---
#set page(width: auto)
// We collapse spaces next to any fullwidth punctuation
你好，
你好。
“你好？”
你好。

// But not if the punctuation is ambiguous on both sides
“你好”
“你好”

--- space-collapsing-korean paged ---
// Korean doesn't collapse spaces on newlines
줄
바꿈

// Unless using fullwidth punctuation
쉼표，
줄 바꿈

--- space-collapsing-cj-bad-edge-case paged ---
// This edge case leaves a space, but is not worth fixing.
水
/**/ 果

--- space-collapsing-cj-strong paged ---
// Test CJ space collapsing with strong emphasis.
*空 *格

*换*
行

// This space still collapses because it is followed by a newline space.
*空格 *
换行

// This space doesn't collapse for the same reason as above.
空格
* 换行*

--- space-collapsing-cj-show-rule paged ---
// Test CJ space collapsing with text show rules.
#show regex("注 释|换行 newline|newline 换行"): set text(red)
#show regex("注释|换行newline|newline换行"): set text(blue)

注//
释

// CJ + Latin does collapse the space
换行
newline

// Latin + CJ does collapse the space
newline
换行

--- space-collapsing-cj-dynamic paged ---
// Test CJ space collapsing with dynamic variables.
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
