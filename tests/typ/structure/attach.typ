// Test list attaching.

---
// Test basic attached list.
Attached to:
- the bottom
- of the paragraph

Next paragraph.

---
// Test attached list without parbreak after it.
// Ensures the par spacing is used below by setting
// super high around spacing.
#set list(around: 100pt)
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
// Test not-attached tight list.
#set list(around: 15pt)
Hello
- A
World

- B
- C

More.

---
// Test that wide lists cannot be attached ...
#set list(around: 15pt, spacing: 15pt)
Hello
- A

- B
World

---
// ... unless really forced to.
Hello
#list(attached: true, tight: false)[A][B]
World
