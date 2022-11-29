// Test list attaching.

---
// Test basic attached list.
Attached to:
- the bottom
- of the paragraph

Next paragraph.

---
// Test that attached list isn't affected by block spacing.
#show list: set block(above: 100pt)
Hello
- A
World
- B

---
// Test non-attached list followed by attached list,
// separated by only word.
Hello

- A

World
- B

---
// Test non-attached tight list.
#set block(spacing: 15pt)
Hello
- A
World

- B
- C

More.

---
// Test that wide lists cannot be ...
#set block(spacing: 15pt)
Hello
- A

- B
World

---
// ... even if forced to.
Hello
#list(tight: false)[A][B]
World
