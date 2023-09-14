
---
#set page(width: 180pt)
#set block(spacing: 5pt)
#set par(justify: true, first-line-indent: 14pt, leading: 5pt)

This text is justified, meaning that spaces are stretched so that the text
forms a "block" with flush edges at both sides.

First line indents and hyphenation play nicely with justified text.

---
// Test that lines with hard breaks aren't justified.
#set par(justify: true)
A B C \
D

---
// Test forced justification with justified break.
A B C #linebreak(justify: true)
D E F #linebreak(justify: true)

---
// Test that there are no hick-ups with justification enabled and
// basically empty paragraph.
#set par(justify: true)
#""

---
// Test that the last line can be shrunk
#set page(width: 155pt)
#set par(justify: true)
This text can be fitted in one line.

---
// Test that runts are avoided when it's not too costly to do so.
#set page(width: 124pt)
#set par(justify: true)
#for i in range(0, 20) {
	"a b c "
}
#"d"

---
// Test that justification cannot lead to a leading space
#set par(justify: true)
#set text(size: 12pt)
#set page(width: 45mm, height: auto)

lorem ipsum 1234, lorem ipsum dolor sit amet

#"  leading whitespace should still be displayed"

---
// Test that justification doesn't break code blocks

#set par(justify: true)

```cpp
int main() {
  printf("Hello world\n");
  return 0;
}
```

